//! Refinery 2.1 Ingestion Gateway.
//!
//! This module provides the `LopdfIngestor` which bridges the gap between
//! raw physical parsing (lopdf) and the refined sequential PdfArena.

use crate::PdfResult;
use crate::arena::{PdfArena, RemappingTable};
use crate::color::ColorSpace;
use crate::error::PdfError;
use crate::font::FontResource;
use crate::handle::Handle;
use crate::object::{Object, PdfName};
use crate::refine::{ParallelRefinery, RefinedObject, commit_to_arena, metadata};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Options for controlling the Refinery ingestion process.
#[derive(Debug, Clone)]
pub struct IngestionOptions {
    /// Enable 2-pass active refinement (Pass 2.0).
    pub active_refinement: bool,
    /// Automatically sublime legacy Document Info to XMP (Pass 4.0).
    pub sublime_metadata: bool,
    /// Policy for ICCBased color space validation.
    pub color_policy: ColorPolicy,
}

impl Default for IngestionOptions {
    fn default() -> Self {
        Self { active_refinement: true, sublime_metadata: true, color_policy: ColorPolicy::Strict }
    }
}

/// Policies for handling color management during ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPolicy {
    /// Require valid ICC profiles and exact mappings.
    Strict,
    /// Allow fallback to device independent profiles on error.
    Relaxed,
}

pub type IngestResult = (PdfArena, Handle<Object>, Option<Handle<Object>>);

pub struct LopdfIngestor;

impl LopdfIngestor {
    /// Ingests a `lopdf::Document` into a new `PdfArena` with specific options.
    pub fn ingest(
        mut_doc: &lopdf::Document,
        options: &IngestionOptions,
    ) -> PdfResult<IngestResult> {
        let mut doc = mut_doc.clone(); // Clone to allow decompression if input is immutable
        doc.decompress();

        let mut arena = PdfArena::new();
        let mut table = RemappingTable::new();

        // Pass 1: Pre-allocate all indirect objects to establish the RemappingTable
        for &id in doc.objects.keys() {
            let handle = arena.alloc_object(Object::Null);
            table.insert((id.0, id.1), handle);
        }

        // Pass 1.5: Resource Pre-scan (New Hardening step)
        let mut font_cache = BTreeMap::new();
        let mut color_cache = BTreeMap::new();

        for (&id, obj) in &doc.objects {
            if let Ok(dict) = obj.as_dict() {
                // Font scanning
                if let Ok(lopdf::Object::Name(name)) = dict.get(b"Type")
                    && name == b"Font"
                    && let Ok(font_res) = FontResource::from_lopdf(id, dict, &doc)
                {
                    let key = font_res.base_font.as_str().to_string();
                    font_cache.insert(key, Arc::new(font_res));
                }

                // ColorSpace scanning (simplified top-level pre-scan)
                if dict.get(b"N").is_ok() && dict.get(b"Alternate").is_ok() {
                    // Likely an ICCBased stream
                    if let Ok(stream) = doc.get_object(id).and_then(|o| o.as_stream())
                        && let Ok(cs) = ColorSpace::from_icc(&stream.content)
                    {
                        color_cache.insert(id, Arc::new(cs));
                    }
                }
            }
        }

        // Pass 2: Parallel refinement with Font Context (respecting options)
        let refined_objects = if options.active_refinement {
            ParallelRefinery::refine_all(&doc.objects, &table, &font_cache)
        } else {
            // Simplified: direct conversion without active normalization
            doc.objects
                .iter()
                .map(|(&id, obj)| (id, RefinedObject::from_lopdf(obj, &table)))
                .collect()
        };

        // Pass 3: Sequential commitment to Arena
        for (id, refined) in refined_objects {
            let handle = table
                .get(&id)
                .copied()
                .ok_or_else(|| PdfError::Other("Handle not found in remapping table".into()))?;
            let obj = commit_to_arena(&mut arena, refined);
            arena.set_object(handle, obj);
        }

        // Pass 4: Metadata sublimation (Info -> XMP) (respecting options)
        if options.sublime_metadata
            && let Ok(info_id) = doc.trailer.get(b"Info").and_then(|o| o.as_reference())
            && let Some(&info_handle) = table.get(&(info_id.0, info_id.1))
            && let Some(Object::Dictionary(dh)) = arena.get_object(info_handle)
            && let Some(dict) = arena.get_dict(dh)
        {
            // Extracting metadata from the arena (simplified for this pass)
            // In a full implementation, we'd convert the Dict<Handle, Object>
            // back to the intermediate form or refine it directly.
            let mut metadata_map = BTreeMap::new();
            for (kh, obj) in dict {
                if let Some(name) = arena.get_name(kh) {
                    // Simplified: only top-level strings for now
                    if let Object::String(s) = obj {
                        metadata_map
                            .insert(name.clone(), crate::refine::RefinedObject::String(s.clone()));
                    }
                }
            }
            let xmp = metadata::info_to_xmp(&metadata_map);
            let xmp_stream = metadata::create_metadata_stream(xmp);
            let xmp_obj = commit_to_arena(&mut arena, xmp_stream);
            let xmp_handle = arena.alloc_object(xmp_obj);

            // Attach to Root
            let root_id = doc.trailer.get(b"Root").and_then(|o| o.as_reference()).ok();
            if let Some(rh) = root_id.and_then(|id| table.get(&(id.0, id.1)))
                && let Some(Object::Dictionary(dh)) = arena.get_object(*rh)
                && let Some(mut root_dict) = arena.get_dict(dh)
            {
                root_dict.insert(
                    arena.intern_name(PdfName::new("Metadata")),
                    Object::Reference(xmp_handle),
                );
                arena.set_dict(dh, root_dict);
            }
        }

        // Finalize: Map the Root
        let root_id = doc
            .trailer
            .get(b"Root")
            .ok() // Convert Result to Option
            .and_then(|o| o.as_reference().ok())
            .ok_or_else(|| PdfError::Ingestion("Missing or invalid Root".into()))?;

        let root_handle = *table
            .get(&(root_id.0, root_id.1))
            .ok_or_else(|| PdfError::Ingestion("Root reference invalid".into()))?;

        let info_id = doc.trailer.get(b"Info").and_then(|o| o.as_reference()).ok();
        let info_handle = info_id.and_then(|id| table.get(&(id.0, id.1)).cloned());

        Ok((arena, root_handle, info_handle))
    }
}
