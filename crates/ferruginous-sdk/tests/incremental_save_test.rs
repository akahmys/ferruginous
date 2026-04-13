#![allow(clippy::all, missing_docs)]
//! Test module

use ferruginous_sdk::core::{Object, Reference};
use ferruginous_sdk::loader::PdfDocument;
use ferruginous_sdk::editor::PdfEditor;
use ferruginous_sdk::xref::{MemoryXRefIndex, XRefEntry};
use std::collections::BTreeMap;
use std::io::Cursor;

#[test]
fn test_incremental_save_structure() {
    // 1. Prepare a base document
    // Obj 1: Catalog
    let base_data = b"1 0 obj << /Type /Catalog /Pages 3 0 R >> endobj\n";
    let mut index = MemoryXRefIndex::default();
    index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 });
    
    let doc = PdfDocument {
        data: base_data.to_vec(),
        xref_index: index,
        last_trailer: ferruginous_sdk::trailer::TrailerInfo { 
            last_xref_offset: 45, // Dummy offset
            trailer_dict: std::sync::Arc::new({
                let mut dict = BTreeMap::new();
                dict.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));
                dict
            }) 
        },
        security: None,
    };

    // 2. Modify and Save
    let mut editor = PdfEditor::new(doc).expect("Failed to create editor");
    
    // Update Obj 1
    let mut new_catalog = BTreeMap::new();
    new_catalog.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
    new_catalog.insert(b"Metadata".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
    editor.update_object(Reference { id: 1, generation: 0 }, Object::new_dict(new_catalog));
    
    // Create Obj 2
    editor.create_object(Object::new_string(b"Metadata Content".to_vec()));

    let mut output = Cursor::new(Vec::new());
    editor.save_incremental(&mut output, false, false).expect("Failed to save incremental");
    
    let result = output.into_inner();
    let result_str = String::from_utf8_lossy(&result);
    
    // 3. Verify structure
    // Check original data Presence
    assert!(result_str.starts_with("1 0 obj << /Type /Catalog /Pages 3 0 R >> endobj\n"));
    
    // Check new objects
    assert!(result_str.contains("2 0 obj\n(Metadata Content)\nendobj\n"));
    
    // Check xref table
    assert!(result_str.contains("xref\n"));
    // Subsections: Obj 1 and Obj 2 are modified/added. They should be in one or two subsections.
    // In our case, 1 and 2 are contiguous, so they should be in one subsection "1 2".
    assert!(result_str.contains("1 2\n"));
    
    // Check trailer and Prev link
    assert!(result_str.contains("/Prev 45"));
    assert!(result_str.contains("/Size 3")); // 0, 1, 2 = 3 objects
    assert!(result_str.contains("startxref\n"));
    assert!(result_str.contains("%%EOF\n"));
    
    // Check 20-byte alignment of xref entries
    // Find the xref table start
    let xref_index = result_str.find("xref\n").unwrap();
    let subsection_index = result_str[xref_index..].find("1 2\n").unwrap() + xref_index;
    let entries_start = subsection_index + 4; // Length of "1 2\n"
    
    let entry1 = &result[entries_start..entries_start+20];
    assert_eq!(entry1.len(), 20);
    assert!(entry1.ends_with(b" \n") || entry1.ends_with(b"\r\n"));
}
