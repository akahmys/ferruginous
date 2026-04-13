use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::core::Object;
use ferruginous_sdk::font::Font;
use ferruginous_sdk::content::{Processor, ContentNode, Operation};
use std::path::Path;
use kurbo::Affine;

fn main() {
    let pdf_path = "../../samples/legacy/writing-mode-1.pdf";
    let data = std::fs::read(pdf_path).expect("failed to read pdf");
    let doc = load_document_structure(&data).expect("failed to load doc");
    let resolver = doc.resolver();

    println!("--- Layout Diagnostics for {} (Page 3) ---", pdf_path);

    let page_tree = doc.page_tree().expect("failed to get page tree");
    let page = page_tree.get_page(2).expect("failed to get page 3"); // 0-indexed
    let bbox = page.media_box_array().unwrap_or([0.0, 0.0, 595.0, 842.0]);
    println!("MediaBox: {:?}", bbox);

    let m_norm = Affine::new([1.0, 0.0, 0.0, -1.0, -bbox[0], bbox[3]]);
    println!("Normalization Matrix (m): {:?}", m_norm);

    let mut processor = Processor::new(page.resources(), Some(bbox), None);
    
    // We want to intercept DrawText commands to see the matrices
    let nodes = page.get_nodes().unwrap();
    
    println!("Analyzing operations...");
    diag_process_nodes(&mut processor, &nodes);
}

fn diag_process_nodes(proc: &mut Processor, nodes: &[ContentNode]) {
    for node in nodes {
        match node {
            ContentNode::Operation(op) => {
                let op_name = String::from_utf8_lossy(&op.operator);
                if op.operator == b"Tj" || op.operator == b"TJ" {
                    println!("Operator: {}", op_name);
                    println!("  Tm (Text Matrix): {:?}", proc.text_state.matrix);
                    if let Some(ref font) = proc.current_font {
                        let fs = proc.text_state.font_size;
                        let th = proc.text_state.horizontal_scaling / 100.0;
                        let rise = proc.text_state.text_rise;
                        
                        let text_extra = Affine::new([fs * th, 0.0, 0.0, -fs, 0.0, rise]);
                        let font_matrix = Affine::new(font.font_matrix);
                        let total = proc.text_state.matrix * text_extra * font_matrix;
                        
                        println!("  FontMatrix: {:?}", font_matrix);
                        println!("  TextExtra: {:?}", text_extra);
                        println!("  Total Local (Tm * Extra * Font): {:?}", total);
                        
                        // Check first glyph
                        if let Object::String(ref s) = op.operands.first().unwrap_or(&Object::String(Arc::new(vec![]))) {
                             if !s.is_empty() {
                                 println!("  First char code: {:02X?}", &s[0..1]);
                             }
                        }
                        println!("----------------------------------");
                    }
                }
                proc.execute_operation(op).ok();
            }
            ContentNode::Block(_, children) => {
                diag_process_nodes(proc, children);
            }
            _ => {}
        }
    }
}

use std::sync::Arc;
