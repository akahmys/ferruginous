//! Visual audit tool for PDF rendering.
use bytes::Bytes;
use ferruginous_sdk::PdfDocument;
use std::path::Path;

/// Main entry point for the visual audit tool.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "samples/constitution_jp.pdf";
    let output_dir = "artifacts";
    std::fs::create_dir_all(output_dir)?;
    let output_path = "artifacts/debug_render_output.png";

    println!("Loading PDF: {pdf_path}");
    let data = Bytes::from(std::fs::read(pdf_path)?);
    let doc = PdfDocument::open(data)?;

    let page_index = 0;
    println!("Rendering Page {page_index} to {output_path}");

    // Use the high-level SDK method which handles the interpreter and backend
    doc.render_page_to_file(page_index, Path::new(output_path))?;

    println!("Render complete. Please check {output_path}");
    Ok(())
}
