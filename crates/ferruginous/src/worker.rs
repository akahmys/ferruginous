use bytes::Bytes;
use ferruginous_render::{FallbackFontType, VelloBackend};
use ferruginous_sdk::PdfDocument;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use vello::Scene;

pub enum WorkerRequest {
    Open { data: Bytes, name: Option<String> },
    RenderPage { index: usize, scale: f64 },
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
