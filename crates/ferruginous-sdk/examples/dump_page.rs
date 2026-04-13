#![allow(clippy::all, missing_docs)]
//! Example module

use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::content::{Processor, parse_content_stream};

fn main() {
    println!("Dumping page traces...");
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 { 
        println!("Usage: dump_page <pdf> <page_number>");
        return; 
    }
    
    let path = &args[1];
    let page_idx = args[2].parse::<usize>().unwrap();
    
    let doc_bytes = std::fs::read(path).unwrap();
    let doc = load_document_structure(&doc_bytes).unwrap();
    let _resolver = doc.resolver();
    
    let pages = doc.page_tree().unwrap();
    match pages.get_page(page_idx - 1) {
        Ok(page) => {
            let contents = page.get_combined_content_data().unwrap_or_default();
            
            let nodes = parse_content_stream(&contents).unwrap();
            let resources = page.resources();
            
            let mut processor = Processor::new(
                resources,
                page.media_box_array(),
                None
            );
            
            processor.process_nodes(&nodes).unwrap();
            println!("Processed {} commands", processor.display_list.len());
        }
        Err(e) => {
            println!("Page not found or error: {}", e);
        }
    }
}
