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
            let _ = tx.send(WorkerResponse::DocumentLoaded { name, num_pages, page_sizes, page_texts });
            Some(doc)
        }
        Err(e) => {
            let _ = tx.send(WorkerResponse::Error(format!("Failed to load PDF: {}", e)));
            None
        }
    }
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
