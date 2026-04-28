use bytes::Bytes;
use ferruginous_render::VelloBackend;
use ferruginous_sdk::PdfDocument;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use vello::Scene;

pub enum WorkerRequest {
    Open(PathBuf),
    RenderPage { index: usize, scale: f64 },
}

pub enum WorkerResponse {
    DocumentLoaded {
        path: PathBuf,
        num_pages: usize,
        page_sizes: Vec<(f64, f64)>, // (width, height)
    },
    PageRendered {
        index: usize,
        scale: f64,
        scene: Arc<Scene>,
    },
    Error(String),
}

pub fn run_worker(rx: Receiver<WorkerRequest>, tx: Sender<WorkerResponse>) {
    let mut current_doc: Option<PdfDocument> = None;

    for request in rx {
        match request {
            WorkerRequest::Open(path) => {
                let data = match std::fs::read(&path) {
                    Ok(d) => Bytes::from(d),
                    Err(e) => {
                        let _ = tx.send(WorkerResponse::Error(format!("Failed to read file: {}", e)));
                        continue;
                    }
                };

                match PdfDocument::open(data) {
                    Ok(doc) => {
                        let num_pages = doc.page_count().unwrap_or(0);
                        let mut page_sizes = Vec::with_capacity(num_pages);
                        for i in 0..num_pages {
                            page_sizes.push(doc.get_page_size(i).unwrap_or((595.0, 842.0)));
                        }
                        
                        current_doc = Some(doc);
                        let _ = tx.send(WorkerResponse::DocumentLoaded {
                            path,
                            num_pages,
                            page_sizes,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(WorkerResponse::Error(format!("Failed to load PDF: {}", e)));
                    }
                }
            }
            WorkerRequest::RenderPage { index, scale } => {
                if let Some(doc) = &current_doc {
                    let (_, p_h) = doc.get_page_size(index).unwrap_or((595.0, 842.0));
                    let mut backend = VelloBackend::new();
                    
                    // Load primary system font for fallback visibility (macOS specific)
                    let font_paths = [
                        "/System/Library/Fonts/ヒラギノ明朝 ProN.ttc",
                        "/System/Library/Fonts/Hiragino Mincho ProN.ttc",
                        "/System/Library/Fonts/ヒラギノ明朝 ProN W3.otf",
                    ];
                    for path in font_paths {
                        if let Ok(data) = std::fs::read(path) {
                            backend.system_fonts.insert("ヒラギノ明朝 ProN".to_string(), std::sync::Arc::new(data));
                            break;
                        }
                    }

                    let initial_transform = kurbo::Affine::new([scale, 0.0, 0.0, -scale, 0.0, p_h * scale]);

                    if let Ok(()) = doc.render_page(index, &mut backend, initial_transform) {
                        let scene = Arc::new(backend.scene().clone());
                        let _ = tx.send(WorkerResponse::PageRendered {
                            index,
                            scale,
                            scene,
                        });
                    } else {
                        let _ = tx.send(WorkerResponse::Error(format!("Failed to render page {}", index)));
                    }
                }
            }
        }
    }
}
