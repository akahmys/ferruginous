//! Refinery 2.1 Concurrent Normalization Pipeline.
//!
//! This module implements the parallel refinement strategy,
//! where objects are refined using `rayon` before being sequentially
//! integrated into the `PdfArena`.

use crate::arena::{PdfArena, RemappingTable};
use crate::handle::Handle;
use crate::object::{Object, PdfName};
use crate::font::FontResource;

use bytes::Bytes;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;

pub mod color;
pub mod font;
pub mod metadata;
pub mod text;

/// A thread-safe intermediate representation of a refined PDF object.
#[derive(Debug, Clone)]
pub enum RefinedObject {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(Bytes),
    Hex(Bytes),
    Name(PdfName),
    Array(Vec<RefinedObject>),
    Dictionary(BTreeMap<PdfName, RefinedObject>),
    Stream(BTreeMap<PdfName, RefinedObject>, Bytes),
    Null,
    Reference(Handle<Object>),
}

impl RefinedObject {
    pub fn as_name(&self) -> Option<&PdfName> {
        match self {
            Self::Name(n) => Some(n),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> Option<&str> {
        self.as_name().map(|n| n.as_str())
    }

    pub fn from_lopdf(obj: &lopdf::Object, table: &RemappingTable) -> Self {
        match obj {
            lopdf::Object::Boolean(b) => Self::Boolean(*b),
            lopdf::Object::Integer(i) => Self::Integer(*i),
            lopdf::Object::Real(f) => Self::Real(*f as f64),
            lopdf::Object::String(s, fmt) => {
                if matches!(fmt, lopdf::StringFormat::Hexadecimal) {
                    Self::Hex(Bytes::copy_from_slice(s))
                } else {
                    Self::String(Bytes::copy_from_slice(s))
                }
            }
            lopdf::Object::Name(n) => Self::Name(PdfName(Bytes::copy_from_slice(n))),
            lopdf::Object::Reference(id) => Self::Reference(
                table.get(&(id.0, id.1)).cloned().unwrap_or(Handle::new(0)),
            ),
            lopdf::Object::Array(arr) => {
                Self::Array(arr.iter().map(|item| Self::from_lopdf(item, table)).collect())
            }
            lopdf::Object::Dictionary(dict) => {
                let mut refined = BTreeMap::new();
                for (k, v) in dict {
                    refined.insert(PdfName(Bytes::copy_from_slice(k)), Self::from_lopdf(v, table));
                }
                Self::Dictionary(refined)
            }
            lopdf::Object::Stream(s) => {
                let mut refined = BTreeMap::new();
                for (k, v) in &s.dict {
                    refined.insert(PdfName(Bytes::copy_from_slice(k)), Self::from_lopdf(v, table));
                }
                Self::Stream(refined, Bytes::copy_from_slice(&s.content))
            }
            lopdf::Object::Null => Self::Null,
        }
    }
}

pub struct ParallelRefinery;

impl ParallelRefinery {
    pub fn refine_all(
        doc: &lopdf::Document,
        table: &RemappingTable,
        handle_fonts: &BTreeMap<u32, Arc<FontResource>>,
        stream_contexts: &BTreeMap<u32, BTreeMap<String, Arc<FontResource>>>,
    ) -> Vec<((u32, u16), RefinedObject, Vec<String>)> {
        doc.objects
            .par_iter()
            .map(|(&id, obj)| {
                let mut issues = Vec::new();
                let refined = Self::refine_recursive(id, obj, table, handle_fonts, stream_contexts, 0, &mut issues);
                (id, refined, issues)
            })
            .collect()
    }

    fn refine_recursive(
        id: (u32, u16),
        obj: &lopdf::Object,
        table: &RemappingTable,
        handle_fonts: &BTreeMap<u32, Arc<FontResource>>,
        stream_contexts: &BTreeMap<u32, BTreeMap<String, Arc<FontResource>>>,
        depth: usize,
        issues: &mut Vec<String>,
    ) -> RefinedObject {
        // Hardening: Recursion depth limit (ISO 32000-2 Clause 7.1)
        if depth > 128 {
            issues.push(format!("Recursion depth limit exceeded for object {:?}", id));
            return RefinedObject::from_lopdf(obj, table);
        }
        match obj {
            lopdf::Object::Dictionary(dict) => {
                let mut refined_dict = BTreeMap::new();
                for (k, v) in dict {
                    refined_dict.insert(
                        PdfName(Bytes::copy_from_slice(k)),
                        Self::refine_recursive(id, v, table, handle_fonts, stream_contexts, depth + 1, issues),
                    );
                }

                // Font Normalization
                if let Some("Font") = refined_dict.get(&PdfName::new("Type")).and_then(|o| o.as_str())
                    && let Some(handle) = table.get(&id) {
                    let resource = handle_fonts.get(&handle.index()).map(|arc| arc.as_ref());
                    return font::normalize_font(refined_dict, resource);
                }

                RefinedObject::Dictionary(refined_dict)
            }
            lopdf::Object::Stream(s) => {
                let mut refined_dict = BTreeMap::new();
                for (k, v) in &s.dict {
                    refined_dict.insert(
                        PdfName(Bytes::copy_from_slice(k)),
                        Self::refine_recursive(id, v, table, handle_fonts, stream_contexts, depth + 1, issues),
                    );
                }

                // Content Stream Restructuring
                let subtype = refined_dict.get(&PdfName::new("Subtype")).and_then(|o| o.as_str());
                let is_form = subtype == Some("Form");
                let is_likely_content = subtype.is_none() || is_form;
                let is_font_data = refined_dict.contains_key(&PdfName::new("Length1")) || refined_dict.contains_key(&PdfName::new("Length2"));

                let mut is_restructured = false;
                let content = if is_likely_content && !is_font_data {
                    if let Some(handle) = table.get(&id) {
                        if let Some(resource_fonts) = stream_contexts.get(&handle.index()) {
                            match s.decompressed_content() {
                                Ok(data) => {
                                    is_restructured = true;
                                    text::restructure_content_stream(&data, resource_fonts)
                                }
                                Err(e) => {
                                    issues.push(format!("Content stream decompression failed for {:?}: {:?}", id, e));
                                    Bytes::copy_from_slice(&s.content)
                                }
                            }
                        } else {
                            Bytes::copy_from_slice(&s.content)
                        }
                    } else {
                        Bytes::copy_from_slice(&s.content)
                    }
                } else {
                    Bytes::copy_from_slice(&s.content)
                };

                if is_restructured {
                    refined_dict.remove(&PdfName::new("Filter"));
                    refined_dict.remove(&PdfName::new("DecodeParms"));
                    refined_dict.insert(PdfName::new("Length"), RefinedObject::Integer(content.len() as i64));
                }

                RefinedObject::Stream(refined_dict, content)
            }
            _ => RefinedObject::from_lopdf(obj, table),
        }
    }
}

pub fn commit_to_arena(arena: &PdfArena, refined: RefinedObject) -> Object {
    match refined {
        RefinedObject::Boolean(b) => Object::Boolean(b),
        RefinedObject::Integer(i) => Object::Integer(i),
        RefinedObject::Real(f) => Object::Real(f),
        RefinedObject::String(s) => Object::String(s),
        RefinedObject::Hex(s) => Object::Hex(s),
        RefinedObject::Name(n) => Object::Name(arena.intern_name(n)),
        RefinedObject::Reference(h) => Object::Reference(h),
        RefinedObject::Array(arr) => {
            let committed = arr.into_iter().map(|item| commit_to_arena(arena, item)).collect();
            Object::Array(arena.alloc_array(committed))
        }
        RefinedObject::Dictionary(dict) => {
            let committed = dict.into_iter().map(|(k, v)| (arena.intern_name(k), commit_to_arena(arena, v))).collect();
            Object::Dictionary(arena.alloc_dict(committed))
        }
        RefinedObject::Stream(dict, bytes) => {
            let committed_dict = dict.into_iter().map(|(k, v)| (arena.intern_name(k), commit_to_arena(arena, v))).collect();
            Object::Stream(arena.alloc_dict(committed_dict), bytes)
        }
        RefinedObject::Null => Object::Null,
    }
}
