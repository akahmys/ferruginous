//! Diagnostic utility for analyzing Japanese text layout.
use bytes::Bytes;
use ferruginous_sdk::PdfDocument;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: diagnose_japanese <pdf_file>");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let data = fs::read(file_path)?;

    let doc = PdfDocument::open(Bytes::from(data)).map_err(|e| format!("{e:?}"))?;
    println!("--- Starting Diagnosis: {file_path} ---");
    let count = doc.page_count().map_err(|e| format!("{e:?}"))?;
    for i in 0..count {
        println!("--- Page {i} ---");
        match doc.extract_text(i) {
            Ok(text) => println!("Text: {text}"),
            Err(e) => eprintln!("--- Page {i} Failed: {e:?} ---"),
        }
    }
    println!("--- Diagnosis Finished Successfully ---");
    Ok(())
}
