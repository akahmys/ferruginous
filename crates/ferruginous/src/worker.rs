#![allow(clippy::collapsible_if, clippy::match_result_ok, clippy::too_many_arguments, clippy::large_enum_variant)]

use bytes::Bytes;
use ferruginous_render::{FallbackFontType, VelloBackend};
use ferruginous_sdk::PdfDocument;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use vello::Scene;

pub enum WorkerRequest {
    Open { data: Bytes, name: Option<String> },
    RenderPage { index: usize, scale: f64 },
    UpdateNode { handle_id: u32, tag: String, alt_text: Option<String> },
    Save {
        path: std::path::PathBuf,
        compress: bool,
        linearize: bool,
        vacuum: bool,
        upgrade_pdf20: bool,
        redaction_zones: Vec<crate::redaction::RedactionZone>,
    },
}

pub enum WorkerResponse {
    DocumentLoaded {
        name: Option<String>,
        num_pages: usize,
        page_sizes: Vec<(f64, f64)>, // (width, height)
        page_texts: Vec<String>,
        ust_root: Option<crate::sidebar::USTNode>,
        audit_findings: Vec<(String, String, String)>, // (checkpoint, severity, message)
    },
    PageRendered {
        index: usize,
        _scale: f64,
        scene: Arc<Scene>,
    },
    AuditFindings {
        findings: Vec<(String, String, String)>,
    },
    DocumentSaved {
        path: std::path::PathBuf,
    },
    Error(String),
}

pub fn run_worker(rx: Receiver<WorkerRequest>, tx: Sender<WorkerResponse>) {
    let mut current_doc: Option<PdfDocument> = None;
    let system_fonts = VelloBackend::load_system_fonts();

    for request in rx {
        match request {
            WorkerRequest::Open { data, name } => current_doc = handle_open(data, name, &tx),
            WorkerRequest::RenderPage { index, scale } => {
                handle_render(&current_doc, index, scale, &tx, Arc::clone(&system_fonts))
            }
            WorkerRequest::UpdateNode { handle_id, tag, alt_text } => {
                handle_update_node(&mut current_doc, handle_id, tag, alt_text, &tx);
            }
            WorkerRequest::Save { path, compress, linearize, vacuum, upgrade_pdf20, redaction_zones } => {
                handle_save(&current_doc, path, compress, linearize, vacuum, upgrade_pdf20, redaction_zones, &tx);
            }
        }
    }
}

use ferruginous_core::{Handle, Object, PdfArena};

fn handle_open(data: Bytes, name: Option<String>, tx: &Sender<WorkerResponse>) -> Option<PdfDocument> {
    match PdfDocument::open(data) {
        Ok(doc) => {
            let num_pages = doc.page_count().unwrap_or(0);
            let mut page_sizes = Vec::with_capacity(num_pages);
            let mut page_texts = Vec::with_capacity(num_pages);
            for i in 0..num_pages {
                page_sizes.push(doc.get_page_size(i).unwrap_or((595.0, 842.0)));
                page_texts.push(doc.extract_text(i).unwrap_or_default());
            }

            let arena = doc.inner().arena();
            let mut next_id = 0;
            let mut ust_root = None;
            let mut audit_findings = Vec::new();

            if let Some(cah) = doc.inner().catalog_handle() {
                if let Some(cadh) = doc.inner().resolve_to_dict(cah).ok() {
                    if let Some(dict) = arena.get_dict(cadh) {
                        let str_root_key = arena.name("StructTreeRoot");
                        if let Some(str_root_obj) = dict.get(&str_root_key) {
                            if let Some(str_root_ref) = str_root_obj.resolve(arena).as_reference() {
                                ust_root = parse_struct_node(arena, str_root_ref, &mut next_id);

                                // Perform true Matterhorn compliance audit
                                let auditor = ferruginous_sdk::structure::MatterhornAuditor::new(arena);
                                if let Ok(findings) = auditor.audit(str_root_ref) {
                                    for f in findings {
                                        audit_findings.push((f.checkpoint, f.severity, f.message));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if ust_root.is_none() {
                // Fallback default node if PDF lacks Structure Tree
                ust_root = Some(crate::sidebar::USTNode {
                    id: 0,
                    tag: "Document".to_string(),
                    title: "PDF Document Catalog (Untagged)".to_string(),
                    alt_text: None,
                    rect: None,
                    handle_id: None,
                    children: Vec::new(),
                });
            }

            let _ = tx.send(WorkerResponse::DocumentLoaded {
                name,
                num_pages,
                page_sizes,
                page_texts,
                ust_root,
                audit_findings,
            });
            Some(doc)
        }
        Err(e) => {
            let _ = tx.send(WorkerResponse::Error(format!("Failed to load PDF: {}", e)));
            None
        }
    }
}

fn parse_struct_node(arena: &PdfArena, handle: Handle<Object>, next_id: &mut usize) -> Option<crate::sidebar::USTNode> {
    let obj = arena.get_object(handle)?;
    let dh = obj.as_dict_handle()?;
    let dict = arena.get_dict(dh)?;

    let type_key = arena.name("Type");
    let s_key = arena.name("S");
    let alt_key = arena.name("Alt");
    let kids_key = arena.name("K");

    let tag = if let Some(s_obj) = dict.get(&s_key) {
        if let Some(name_h) = s_obj.resolve(arena).as_name() {
            arena.get_name(name_h).map(|n| n.as_str().to_string()).unwrap_or_else(|| "P".to_string())
        } else {
            "P".to_string()
        }
    } else {
        let type_val = dict.get(&type_key).and_then(|t| t.resolve(arena).as_name());
        if let Some(tv) = type_val {
            if arena.get_name(tv).map(|n| n.as_str() == "StructTreeRoot").unwrap_or(false) {
                "Document".to_string()
            } else {
                "P".to_string()
            }
        } else {
            "P".to_string()
        }
    };

        let title = tag.clone();
    let alt_text = if let Some(alt_obj) = dict.get(&alt_key) {
        if let Some(bytes) = alt_obj.resolve(arena).as_string() {
            String::from_utf8(bytes.to_vec()).ok()
        } else {
            None
        }
    } else {
        None
    };

    let rect = if let Some(bbox_obj) = dict.get(&arena.name("BBox")) {
        if let Some(array_h) = bbox_obj.resolve(arena).as_array() {
            if let Some(arr) = arena.get_array(array_h) {
                if arr.len() == 4 {
                    let x1 = arr[0].resolve(arena).as_f64().unwrap_or(0.0) as f32;
                    let y1 = arr[1].resolve(arena).as_f64().unwrap_or(0.0) as f32;
                    let x2 = arr[2].resolve(arena).as_f64().unwrap_or(0.0) as f32;
                    let y2 = arr[3].resolve(arena).as_f64().unwrap_or(0.0) as f32;
                    Some([x1, y1, x2, y2])
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let id = *next_id;
    *next_id += 1;

    let mut children = Vec::new();
    if let Some(kids) = dict.get(&kids_key) {
        match kids.resolve(arena) {
            Object::Array(ah) => {
                if let Some(array) = arena.get_array(ah) {
                    for kid in array {
                        if let Some(kid_ref) = kid.resolve(arena).as_reference() {
                            if let Some(child_node) = parse_struct_node(arena, kid_ref, next_id) {
                                children.push(child_node);
                            }
                        }
                    }
                }
            }
            Object::Reference(kid_ref) => {
                if let Some(child_node) = parse_struct_node(arena, kid_ref, next_id) {
                    children.push(child_node);
                }
            }
            _ => {} // Ignore leaves like MCR or OBJR for structure tree visualization
        }
    }

    Some(crate::sidebar::USTNode {
        id,
        tag,
        title,
        alt_text,
        rect,
        handle_id: Some(handle.index()),
        children,
    })
}

fn handle_render(
    doc_opt: &Option<PdfDocument>,
    index: usize,
    scale: f64,
    tx: &Sender<WorkerResponse>,
    system_fonts: Arc<std::collections::BTreeMap<FallbackFontType, Arc<Vec<u8>>>>,
) {
    let Some(doc) = doc_opt else { return };
    let (_, p_h) = doc.get_page_size(index).unwrap_or((595.0, 842.0));
    let mut backend = VelloBackend::new(system_fonts);
    let initial_transform = kurbo::Affine::new([scale, 0.0, 0.0, -scale, 0.0, p_h * scale]);

    if let Ok(()) = doc.render_page(index, &mut backend, initial_transform) {
        let scene = Arc::new(backend.scene().clone());
        let _ = tx.send(WorkerResponse::PageRendered { index, _scale: scale, scene });
    } else {
        let _ = tx.send(WorkerResponse::Error(format!("Failed to render page {}", index)));
    }
}

fn handle_update_node(
    doc_opt: &mut Option<PdfDocument>,
    handle_id: u32,
    tag: String,
    alt_text: Option<String>,
    tx: &Sender<WorkerResponse>,
) {
    let Some(doc) = doc_opt else { return };
    let arena = doc.inner().arena();
    let handle = Handle::<Object>::new(handle_id);

    if let Some(Object::Dictionary(dh)) = arena.get_object(handle) {
        if let Some(mut dict) = arena.get_dict(dh) {
            // Update tag Subtype /S
            let s_key = arena.name("S");
            dict.insert(s_key, Object::Name(arena.name(&tag)));

            // Update Alt-text
            let alt_key = arena.name("Alt");
            if let Some(alt) = alt_text {
                dict.insert(alt_key, Object::String(bytes::Bytes::from(alt)));
            } else {
                dict.remove(&alt_key);
            }
            arena.set_dict(dh, dict);
        }
    }

    // Run Matterhorn compliance audit on updated tree
    let mut findings = Vec::new();
    if let Some(cah) = doc.inner().catalog_handle() {
        if let Some(cadh) = doc.inner().resolve_to_dict(cah).ok() {
            if let Some(dict) = arena.get_dict(cadh) {
                let str_root_key = arena.name("StructTreeRoot");
                if let Some(str_root_obj) = dict.get(&str_root_key) {
                    if let Some(str_root_ref) = str_root_obj.resolve(arena).as_reference() {
                        let auditor = ferruginous_sdk::structure::MatterhornAuditor::new(arena);
                        if let Ok(audit_res) = auditor.audit(str_root_ref) {
                            for f in audit_res {
                                findings.push((f.checkpoint, f.severity, f.message));
                            }
                        }
                    }
                }
            }
        }
    }
    let _ = tx.send(WorkerResponse::AuditFindings { findings });
}

fn handle_save(
    doc_opt: &Option<PdfDocument>,
    path: std::path::PathBuf,
    compress: bool,
    linearize: bool,
    vacuum: bool,
    upgrade_pdf20: bool,
    redaction_zones: Vec<crate::redaction::RedactionZone>,
    tx: &Sender<WorkerResponse>,
) {
    let Some(doc) = doc_opt else {
        let _ = tx.send(WorkerResponse::Error("No document loaded to save".to_string()));
        return;
    };

    // 1. Group redaction zones by page index
    let mut page_redactions: std::collections::BTreeMap<usize, Vec<[f32; 4]>> = std::collections::BTreeMap::new();
    for zone in redaction_zones {
        let rect_arr = [zone.rect.min.x, zone.rect.min.y, zone.rect.max.x, zone.rect.max.y];
        page_redactions.entry(zone.page_index).or_default().push(rect_arr);
    }

    // 2. Apply physical stream sanitization to each page mutably
    for (page_idx, rects) in page_redactions {
        if let Err(e) = ferruginous_sdk::apply_physical_redaction_to_page(doc.inner(), page_idx, &rects) {
            let _ = tx.send(WorkerResponse::Error(format!("Failed physically redacting page {}: {}", page_idx, e)));
            return;
        }
    }

    let version = if upgrade_pdf20 { "2.0" } else { "1.7" };
    let options = ferruginous_sdk::SaveOptions {
        compress,
        compression_level: 6,
        vacuum,
        ..ferruginous_sdk::SaveOptions::default()
    };

    let res = if linearize {
        doc.save_linearized(&path, version, &options)
    } else {
        doc.save_with_options(&path, version, &options)
    };

    match res {
        Ok(()) => {
            let _ = tx.send(WorkerResponse::DocumentSaved { path });
        }
        Err(e) => {
            let _ = tx.send(WorkerResponse::Error(format!("Failed to save PDF: {}", e)));
        }
    }
}
