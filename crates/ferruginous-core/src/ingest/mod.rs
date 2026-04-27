//! Ingestor module for transitioning lopdf::Document to PdfArena.

use crate::arena::{PdfArena, RemappingTable};
use crate::handle::Handle;
use crate::object::Object;
use crate::security::SecurityHandler;
use crate::error::PdfError;
use crate::Document;
use crate::refine::ParallelRefinery;



mod discovery;
pub use discovery::*;

/// Policy for color validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPolicy {
    Strict,
    Relaxed,
}

/// Options for document ingestion.
#[derive(Debug, Clone)]
pub struct IngestionOptions {
    pub active_refinement: bool,
    pub sublime_metadata: bool,
    pub color_policy: ColorPolicy,
}

impl Default for IngestionOptions {
    fn default() -> Self {
        Self {
            active_refinement: true,
            sublime_metadata: true,
            color_policy: ColorPolicy::Strict,
        }
    }
}

pub struct Ingestor;

/// Result of a document ingestion.
pub struct IngestedDocument {
    pub arena: PdfArena,
    pub root: Handle<Object>,
    pub info: Option<Handle<Object>>,
    pub issues: Vec<String>,
}

impl Ingestor {
    /// Ingests a lopdf::Document into a new PdfArena and returns a refined Document.
    ///
    /// The ingestion process is a three-pass system:
    /// 1. **Pass 0 (Normalization)**: Decrypts all objects and cleans the physical structure.
    /// 2. **Pass 1 (Inhalation)**: Maps lopdf objects to PdfArena Handles, decoupling from physical offsets.
    /// 3. **Pass 1.5 (Context Discovery)**: Discovers font resources and maps them to content streams.
    /// 4. **Pass 2 (Refinement)**: Parallel processing of page content and metadata.
    pub fn ingest(doc: &mut lopdf::Document, _options: &IngestionOptions) -> crate::PdfResult<IngestedDocument> {
        let arena = PdfArena::new();
        let mut table = RemappingTable::new();

        // Pass 0: Manual Decryption if lopdf's built-in decryption is incomplete/unsupported
        Self::perform_pass_0_decryption(doc)?;

        // Pass 1: Inhale all objects into the Arena
        for &id in doc.objects.keys() {
            let handle = arena.alloc_object(Object::Null);
            table.insert(id, handle);
        }

        for (&id, obj) in &doc.objects {
            let handle = table.get(&id).cloned().ok_or_else(|| {
                PdfError::Ingestion {
                    context: "Pass 1 Inhalation".into(),
                    message: format!("Missing handle for object {:?}", id).into(),
                }
            })?;
            let raw_obj = Object::from_lopdf(obj, &arena, &table);
            arena.set_object(handle, raw_obj);
        }

        // Pass 1.5: Font Discovery & Contextual Mapping
        let root_id = doc.trailer.get(b"Root").and_then(|o| o.as_reference()).ok();
        let info_id = doc.trailer.get(b"Info").and_then(|o| o.as_reference()).ok();
        let root_handle = root_id.and_then(|id| table.get(&id)).cloned().unwrap_or(Handle::new(0));
        let info_handle = info_id.and_then(|id| table.get(&id)).cloned();
        
        let temp_doc = Document::new(arena.clone(), root_handle, info_handle);
        let handle_font_cache = discover_fonts(&arena, &temp_doc);
        let stream_contexts = map_stream_contexts(&arena, &handle_font_cache);

        println!("Pass 1.5: Discovered {} fonts and {} contextual stream mappings.", handle_font_cache.len(), stream_contexts.len());

        let mut all_issues = Vec::new();
        if _options.active_refinement {
            // Pass 2: Active Refinement
            println!("Pass 2: Active Refinement...");
            let refined_results =
                ParallelRefinery::refine_all(doc, &table, &handle_font_cache, &stream_contexts);

            // Pass 3: Sequential Integration
            for (id, refined, mut issues) in refined_results {
                let handle = table.get(&id).cloned().ok_or_else(|| {
                    PdfError::Ingestion {
                        context: "Pass 3 Integration".into(),
                        message: format!("Missing handle for refined object {:?}", id).into(),
                    }
                })?;
                let committed = crate::refine::commit_to_arena(&arena, refined);
                arena.set_object(handle, committed);
                all_issues.append(&mut issues);
            }
        }

        Ok(IngestedDocument {
            arena,
            root: root_handle,
            info: info_handle,
            issues: all_issues,
        })
    }

    /// Performs "Pass 0" normalization by decrypting the raw PDF objects in-place.
    ///
    /// This is required because Acrobat (and other strict viewers) will fail with Error 135
    /// if a document is saved with decrypted objects but still contains an `/Encrypt` trailer entry.
    /// After decryption, this method explicitly removes the `/Encrypt` dictionary to satisfy
    /// Adobe fidelity requirements.
    fn perform_pass_0_decryption(doc: &mut lopdf::Document) -> crate::PdfResult<()> {
        let mut security_handler = None;
        
        if let Ok(encrypt_dict_obj) = doc.trailer.get(b"Encrypt") {
            let encrypt_obj = if let Ok(id) = encrypt_dict_obj.as_reference() {
                doc.objects.get(&id)
            } else {
                Some(encrypt_dict_obj)
            };

            if let Some(lopdf::Object::Dictionary(dict)) = encrypt_obj {
                let v_val = dict.get(b"V").and_then(|o| o.as_i64()).unwrap_or(0);
                let r_val = dict.get(b"R").and_then(|o| o.as_i64()).unwrap_or(0);
                
                if v_val == 4 && r_val == 4 {
                    let o_str = dict.get(b"O").and_then(|o| o.as_str()).unwrap_or(&[]);
                    let u_str = dict.get(b"U").and_then(|o| o.as_str()).unwrap_or(&[]);
                    let p_val = dict.get(b"P").and_then(|o| o.as_i64()).unwrap_or(0) as i32;
                    let file_id = doc.trailer.get(b"ID")
                        .and_then(|o| o.as_array())
                        .map(|a| a.first().and_then(|o| o.as_str().ok()).unwrap_or(&[]))
                        .unwrap_or(&[]);
                    
                    if let Ok(handler) = SecurityHandler::new_v4("", o_str, u_str, p_val, file_id) {
                        security_handler = Some(handler);
                    }
                } else if v_val == 5 && r_val == 5 {
                    let mut file_id = &[][..];
                    if let Ok(id_array) = doc.trailer.get(b"ID")
                        && let Ok(arr) = id_array.as_array()
                        && let Some(first) = arr.first()
                        && let Ok(s) = first.as_str() {
                        file_id = s;
                    }
                    if let Ok(handler) = SecurityHandler::new_v5("", "", file_id) {
                        security_handler = Some(handler);
                    }
                }
            }
        }

        if let Some(handler) = security_handler {
            let ids: Vec<lopdf::ObjectId> = doc.objects.keys().cloned().collect();
            for id in ids {
                if let Some(obj) = doc.objects.get_mut(&id) {
                    Self::decrypt_object_stacked(obj, id, &handler)?;
                }
            }
            doc.trailer.remove(b"Encrypt");
        }
        Ok(())
    }

    /// Iteratively traverses a PDF object tree and decrypts all strings and streams.
    ///
    /// This method is **RR-15 Rule 6 compliant** (no recursion). It uses an explicit stack
    /// to walk through Dictionaries and Arrays. This is critical for PDF documents which
    /// can have arbitrary nesting depths that would otherwise cause a stack overflow.
    fn decrypt_object_stacked(
        root_obj: &mut lopdf::Object,
        id: lopdf::ObjectId,
        handler: &SecurityHandler,
    ) -> crate::PdfResult<()> {
        let mut stack = vec![root_obj];

        while let Some(obj) = stack.pop() {
            match obj {
                lopdf::Object::String(s, _) => {
                    let decrypted = handler.decrypt_bytes(s, id.0, id.1)?;
                    *s = decrypted;
                }
                lopdf::Object::Stream(stream) => {
                    let decrypted = handler.decrypt_bytes(&stream.content, id.0, id.1)?;
                    stream.content = decrypted;
                }
                lopdf::Object::Array(arr) => {
                    for item in arr {
                        stack.push(item);
                    }
                }
                lopdf::Object::Dictionary(dict) => {
                    for (_, item) in dict.iter_mut() {
                        stack.push(item);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
