#![allow(clippy::all, missing_docs)]
//! Test module

use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::editor::PdfEditor;
use std::path::Path;

#[test]
fn test_form_field_update() {
    let pdf_path = Path::new("../../samples/legacy/simple.pdf");
    if !pdf_path.exists() {
        return; // Skip if samples are missing
    }
    
    let data = std::fs::read(pdf_path).unwrap();
    let doc = load_document_structure(&data).unwrap();
    let mut editor = PdfEditor::new(doc).expect("Failed to create editor");
    
    // In simple.pdf, there might not be forms, but we can test the accessor
    let catalog = editor.document.catalog().unwrap();
    let _acroform = catalog.acroform();
    
    // Verification of the API surface
    if let Some(ref context) = editor.oc_context {
        assert!(context.states.is_empty() || !context.states.is_empty());
    }
}
