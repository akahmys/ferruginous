//! Diagnostic utility for analyzing Japanese text layout and matrix transformations.
//!
//! (ISO 32000-2:2020 Clause 9.7.4.3)

use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::PdfResult;
use std::env;
use std::fs;

fn main() -> PdfResult<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: diagnose_japanese <pdf_file>");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let data = fs::read(file_path).map_err(|e| ferruginous_sdk::PdfError::ResourceError(e.to_string()))?;

    let doc = load_document_structure(&data)?;
    let page_tree = doc.page_tree()?;
    println!("--- Starting Diagnosis: {} ---", file_path);
    let count = page_tree.get_count();
    for i in 0..count {
        println!("--- Page {} ---", i);
        let page = page_tree.get_page(i)?;
        match page.get_display_list() {
            Ok(_) => println!("--- Page {} Finished Successfully ---", i),
            Err(e) => eprintln!("--- Page {} Failed: {:?} ---", i, e),
        }
    }
    println!("--- Diagnosis Finished Successfully ---");
    Ok(())
}
