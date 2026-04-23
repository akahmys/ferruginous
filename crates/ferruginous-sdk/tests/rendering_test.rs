//! Integration tests for the Ferruginous SDK rendering pipeline.
use bytes::Bytes;
use ferruginous_sdk::PdfDocument;
use std::path::Path;

#[tokio::test]
async fn test_render_sample_pdf() {
    let pdf_path = Path::new("../../samples/standard/Simple PDF 2.0 file.pdf");
    assert!(pdf_path.exists(), "Sample PDF not found at {pdf_path:?}");

    let data = std::fs::read(pdf_path).expect("Failed to read sample PDF");
    let doc = PdfDocument::open(Bytes::from(data)).expect("Failed to open PDF document");

    let output_path = Path::new("test_output.png");

    // Render page 0 (1st page)
    doc.render_page_to_file(0, output_path).expect("Failed to render page to file");

    assert!(output_path.exists(), "Output PNG file was not created");

    let metadata = std::fs::metadata(output_path).expect("Failed to get metadata for output PNG");
    assert!(metadata.len() > 0, "Output PNG file is empty");

    println!("Successfully rendered sample PDF to {output_path:?}");
}
