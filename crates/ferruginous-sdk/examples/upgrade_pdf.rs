//! Example for upgrading a PDF to 2.0.
use bytes::Bytes;
use ferruginous_sdk::PdfDocument;
use std::path::Path;

/// Main entry point for the PDF upgrade example.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_path = "samples/bokutokitan.pdf";
    let output_path = "samples/bokutokitan.v20.pdf";

    println!("Loading {input_path}...");
    let data = Bytes::from(std::fs::read(input_path)?);
    let doc = PdfDocument::open(data).map_err(|e| format!("{e:?}"))?;

    println!("Upgrading to PDF 2.0 and saving to {output_path}...");
    doc.save_as_version(Path::new(output_path), "2.0").map_err(|e| format!("{e:?}"))?;

    println!("Success!");
    Ok(())
}
