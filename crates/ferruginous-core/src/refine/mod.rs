//! Refinery 2.1 Concurrent Normalization Pipeline.
//!
//! This module implements the "Option B" parallel refinement strategy,
//! where objects are refined using `rayon` before being sequentially
//! integrated into the `PdfArena`.

use crate::arena::{PdfArena, RemappingTable};
use crate::handle::Handle;
use crate::object::{Object, PdfName};

use bytes::Bytes;
use rayon::prelude::*;
use std::collections::BTreeMap;

pub mod color;
pub mod font;
pub mod metadata;
pub mod text;

use crate::font::FontResource;
use std::sync::Arc;

/// A thread-safe intermediate representation of a refined PDF object.
/// This representation handles recursive structures (Arrays, Dicts) inline
/// before they are pooled in the Arena.
#[derive(Debug, Clone)]
pub enum RefinedObject {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(Bytes),
    Name(PdfName),
    Array(Vec<RefinedObject>),
    Dictionary(BTreeMap<PdfName, RefinedObject>),
    Stream(BTreeMap<PdfName, RefinedObject>, Bytes),
    Null,
    Reference(Handle<Object>),
}

impl RefinedObject {
    /// Performs a shallow conversion from a lopdf object.
    pub fn from_lopdf(obj: &lopdf::Object, table: &RemappingTable) -> Self {
        match obj {
            lopdf::Object::Boolean(b) => RefinedObject::Boolean(*b),
            lopdf::Object::Integer(i) => RefinedObject::Integer(*i),
            lopdf::Object::Real(f) => RefinedObject::Real(*f as f64),
            lopdf::Object::String(s, _) => RefinedObject::String(Bytes::copy_from_slice(s)),
            lopdf::Object::Name(n) => RefinedObject::Name(PdfName(Bytes::copy_from_slice(n))),
            lopdf::Object::Reference(id) => {
                let handle = table.get(&(id.0, id.1)).cloned().unwrap_or(Handle::new(0));
                RefinedObject::Reference(handle)
            }
            lopdf::Object::Array(arr) => {
                RefinedObject::Array(arr.iter().map(|item| Self::from_lopdf(item, table)).collect())
            }
            lopdf::Object::Dictionary(dict) => {
                let mut refined_dict = BTreeMap::new();
                for (k, v) in dict {
                    refined_dict
                        .insert(PdfName(Bytes::copy_from_slice(k)), Self::from_lopdf(v, table));
                }
                RefinedObject::Dictionary(refined_dict)
            }
            lopdf::Object::Stream(stream) => {
                let mut refined_dict = BTreeMap::new();
                for (k, v) in &stream.dict {
                    refined_dict
                        .insert(PdfName(Bytes::copy_from_slice(k)), Self::from_lopdf(v, table));
                }
                RefinedObject::Stream(refined_dict, Bytes::copy_from_slice(&stream.content))
            }
            lopdf::Object::Null => RefinedObject::Null,
        }
    }
}

const MAX_RECURSION_DEPTH: u32 = 64;

pub struct ParallelRefinery;

impl ParallelRefinery {
    /// Refines a collection of lopdf objects in parallel.
    pub fn refine_all(
        objects: &BTreeMap<(u32, u16), lopdf::Object>,
        table: &RemappingTable,
        fonts: &BTreeMap<String, Arc<FontResource>>,
    ) -> Vec<((u32, u16), RefinedObject)> {
        objects
            .par_iter()
            .map(|(&id, obj)| {
                let refined = Self::refine_recursive_depth(obj, table, fonts, 0);
                (id, refined)
            })
            .collect()
    }

    #[allow(dead_code)]
    pub(crate) fn refine_recursive(
        obj: &lopdf::Object,
        table: &RemappingTable,
        fonts: &BTreeMap<String, Arc<FontResource>>,
    ) -> RefinedObject {
        Self::refine_recursive_depth(obj, table, fonts, 0)
    }

    fn refine_recursive_depth(
        obj: &lopdf::Object,
        table: &RemappingTable,
        fonts: &BTreeMap<String, Arc<FontResource>>,
        depth: u32,
    ) -> RefinedObject {
        if depth > MAX_RECURSION_DEPTH {
            return RefinedObject::Null;
        }

        match obj {
            lopdf::Object::Boolean(b) => RefinedObject::Boolean(*b),
            lopdf::Object::Integer(i) => RefinedObject::Integer(*i),
            lopdf::Object::Real(f) => RefinedObject::Real(*f as f64),
            lopdf::Object::String(s, _) => RefinedObject::String(text::refine_string(s)),
            lopdf::Object::Name(n) => {
                let name = PdfName(Bytes::copy_from_slice(n));
                if let Some(refined) = color::normalize_colorspace(&name) {
                    refined
                } else {
                    RefinedObject::Name(name)
                }
            }
            lopdf::Object::Reference(id) => {
                let handle = table.get(&(id.0, id.1)).cloned().unwrap_or(Handle::new(0)); // Should not happen with valid table
                RefinedObject::Reference(handle)
            }
            lopdf::Object::Array(arr) => {
                let refined_arr = arr
                    .iter()
                    .map(|item| Self::refine_recursive_depth(item, table, fonts, depth + 1))
                    .collect();
                RefinedObject::Array(refined_arr)
            }
            lopdf::Object::Dictionary(dict) => {
                let mut refined_dict = BTreeMap::new();
                for (k, v) in dict {
                    refined_dict.insert(
                        PdfName(Bytes::copy_from_slice(k)),
                        Self::refine_recursive_depth(v, table, fonts, depth + 1),
                    );
                }

                // Apply Font normalization if it's a font dictionary
                let type_key = PdfName(Bytes::from_static(b"Type"));
                let font_name = PdfName(Bytes::from_static(b"Font"));
                if let Some(RefinedObject::Name(t)) = refined_dict.get(&type_key)
                    && t == &font_name
                {
                    return font::normalize_font(refined_dict);
                }

                RefinedObject::Dictionary(refined_dict)
            }
            lopdf::Object::Stream(s) => {
                let mut refined_dict = BTreeMap::new();
                for (k, v) in &s.dict {
                    refined_dict.insert(
                        PdfName(Bytes::copy_from_slice(k)),
                        Self::refine_recursive_depth(v, table, fonts, depth + 1),
                    );
                }

                // Active Refinement: Restructure Content Stream if it's likely a Page or XObject content.
                // Hardening: Skip if it contains font-specific (Length1/2/3) or image-specific keys.
                let length_key = PdfName::new("Length");
                let subtype_key = PdfName::new("Subtype");
                let l1 = PdfName::new("Length1");
                let l2 = PdfName::new("Length2");

                let is_likely_content = refined_dict.contains_key(&length_key)
                    && !refined_dict.contains_key(&subtype_key)
                    && !refined_dict.contains_key(&l1)
                    && !refined_dict.contains_key(&l2);

                let content = if is_likely_content {
                    text::restructure_content_stream(&s.content, fonts)
                } else {
                    Bytes::copy_from_slice(&s.content)
                };

                RefinedObject::Stream(refined_dict, content)
            }
            lopdf::Object::Null => RefinedObject::Null,
        }
    }
}

/// Helper to convert RefinedObject back to Arena-backed Object.
/// This happens sequentially.
pub fn commit_to_arena(arena: &mut PdfArena, refined: RefinedObject) -> Object {
    match refined {
        RefinedObject::Boolean(b) => Object::Boolean(b),
        RefinedObject::Integer(i) => Object::Integer(i),
        RefinedObject::Real(f) => Object::Real(f),
        RefinedObject::String(s) => Object::String(s),
        RefinedObject::Name(n) => Object::Name(arena.intern_name(n)),
        RefinedObject::Array(arr) => {
            let items = arr.into_iter().map(|i| commit_to_arena(arena, i)).collect();
            Object::Array(arena.alloc_array(items))
        }
        RefinedObject::Dictionary(dict) => {
            let mut items = BTreeMap::new();
            for (k, v) in dict {
                let kh = arena.intern_name(k);
                items.insert(kh, commit_to_arena(arena, v));
            }
            Object::Dictionary(arena.alloc_dict(items))
        }
        RefinedObject::Stream(dict, bytes) => {
            let mut items = BTreeMap::new();
            for (k, v) in dict {
                let kh = arena.intern_name(k);
                items.insert(kh, commit_to_arena(arena, v));
            }
            Object::Stream(arena.alloc_dict(items), bytes)
        }
        RefinedObject::Null => Object::Null,
        RefinedObject::Reference(h) => Object::Reference(h),
    }
}
