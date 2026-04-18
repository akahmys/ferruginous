#![allow(clippy::all, missing_docs)]
//! Layout diagnostics utility.

use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::core::Object;
use ferruginous_sdk::content::{Processor, ContentNode, parse_content_stream};
use kurbo::Affine;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: diag_layout <pdf_path> [page_index]");
        return Ok(());
    }
    let pdf_path = &args[1];
    let target_page = args.get(2).and_then(|s| s.parse::<usize>().ok());

    let data = std::fs::read(pdf_path)?;
    let doc = load_document_structure(&data)?;
    
    let page_tree = doc.page_tree()?;
    let count = page_tree.get_count();
    
    for i in 0..count {
        if let Some(target) = target_page {
            if i != target { continue; }
        }
        
        println!("--- Layout Diagnostics for {} (Page {}) ---", pdf_path, i + 1);
        let page = page_tree.get_page(i)?;
        let bbox = page.media_box_array().unwrap_or([0.0, 0.0, 595.0, 842.0]);
        println!("MediaBox: {bbox:?}");

        let mut processor = Processor::new(page.resources(), Some(bbox), None);
        let content_data = page.get_combined_content_data().unwrap_or_default();
        let nodes = parse_content_stream(&content_data)?;
        
        println!("Analyzing operations...");
        diag_process_nodes(&mut processor, &nodes);
    }
    Ok(())
}

fn diag_process_nodes(proc: &mut Processor, nodes: &[ContentNode]) {
    for node in nodes {
        match node {
            ContentNode::Operation(op) => {
                let op_name = String::from_utf8_lossy(&op.operator);
                if op.operator == b"Tj" || op.operator == b"TJ" {
                    print_text_diag(proc, op, &op_name);
                }
                let _ = proc.execute_operation(op);
            }
            ContentNode::Block(_, children) => {
                diag_process_nodes(proc, children);
            }
            _ => {}
        }
    }
}

fn print_text_diag(proc: &Processor, op: &ferruginous_sdk::content::Operation, op_name: &str) {
    println!("Operator: {op_name}");
    println!("  Tm (Text Matrix): {:?}", proc.text_state.matrix);
    if let Some(ref font) = proc.current_font {
        let fs = proc.text_state.font_size;
        let th = proc.text_state.horizontal_scaling / 100.0;
        let rise = proc.text_state.text_rise;
        
        let text_extra = Affine::new([fs * th, 0.0, 0.0, fs, 0.0, rise]);
        let font_matrix = Affine::new(font.font_matrix);
        let total = proc.text_state.matrix * text_extra * font_matrix;
        
        println!("  FontMatrix: {font_matrix:?}");
        println!("  TextExtra: {text_extra:?}");
        println!("  Total Local (Tm * Extra * Font): {total:?}");
        
        if let Some(Object::String(s)) = op.operands.first() {
             if !s.is_empty() {
                 println!("  First char code: {:02X?}", &s[0..1]);
             }
        }
        println!("----------------------------------");
    }
}
