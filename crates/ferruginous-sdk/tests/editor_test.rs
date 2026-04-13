#![allow(clippy::all, missing_docs)]
//! Test module

//! Integration tests for PDF document editor.
use ferruginous_sdk::core::{Object, Reference, Resolver};
use ferruginous_sdk::loader::PdfDocument;
use ferruginous_sdk::editor::PdfEditor;
use ferruginous_sdk::xref::{MemoryXRefIndex, XRefEntry};
use std::collections::BTreeMap;

#[test]
fn test_editor_full_workflow() {
    // 1. Prepare a mock document
    let mut index = MemoryXRefIndex::default();
    // Obj 1: Catalog
    index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 });
    
    let mut catalog_dict = BTreeMap::new();
    catalog_dict.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
    
    // We need a way to mock the data for resolution. 
    // Since PdfResolver parses from bytes, we should provide bytes.
    // "1 0 obj << /Type /Catalog >> endobj" is at offset 0.
    let data = b"1 0 obj << /Type /Catalog >> endobj\n";
    
    let doc = PdfDocument {
        data: data.to_vec(),
        xref_index: index,
        last_trailer: ferruginous_sdk::trailer::TrailerInfo { 
            last_xref_offset: 0, 
            trailer_dict: std::sync::Arc::new({
                let mut dict = BTreeMap::new();
                dict.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));
                dict
            }) 
        },
        security: None,
    };

    // 2. Initialize Editor
    let mut editor = PdfEditor::new(doc).expect("Failed to create editor");
    let ref1 = Reference { id: 1, generation: 0 };
    
    // Resolve original
    let obj1 = editor.get_object(&ref1).expect("Failed to resolve original catalog");
    assert!(matches!(obj1, Object::Dictionary(_)));

    // 3. Update existing object (Add Metadata reference to Catalog)
    let mut new_catalog = catalog_dict.clone();
    let metadata_ref = Reference { id: 2, generation: 0 };
    new_catalog.insert(b"Metadata".to_vec(), Object::Reference(metadata_ref));
    
    editor.update_object(ref1, Object::new_dict(new_catalog));
    
    // Verify update
    let updated_obj = editor.get_object(&ref1).unwrap();
    if let Object::Dictionary(dict) = updated_obj {
        assert!(dict.contains_key(b"Metadata".as_slice()));
    } else {
        panic!("Expected dictionary");
    }

    // 4. Create new object (The Metadata stream)
    let meta_stream = Object::new_stream(BTreeMap::new(), b"<xmp>Test</xmp>".to_vec());
    let new_ref = editor.create_object(meta_stream).expect("Failed to create meta object");
    
    assert_eq!(new_ref.id, 2); // next_id should be 2 since max was 1
    
    let resolved_meta = editor.get_object(&new_ref).expect("Failed to resolve meta object");
    if let Object::Stream(_, data) = resolved_meta {
        assert_eq!(data.as_ref(), b"<xmp>Test</xmp>");
    } else {
        panic!("Expected stream");
    }
}

#[test]
fn test_save_incremental_integrity() {
    use std::io::Cursor;
    
    // 1. Setup base doc (with correct offsets)
    // %PDF-1.7\n (9)
    // 1 0 obj\n<< /Type /Catalog >>\nendobj\n (36)
    // Total offset for xref is 45.
    let data = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000009 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n45\n%%EOF\n";
    let doc = ferruginous_sdk::loader::load_document_structure(data).expect("Base load fail");
    
    // 2. Edit
    let mut editor = PdfEditor::new(doc).expect("Failed to create editor");
    let new_obj = Object::Integer(999);
    let new_ref = editor.create_object(new_obj).unwrap();
    
    // 3. Save Incremental
    let mut buffer = Cursor::new(Vec::new());
    editor.save_incremental(&mut buffer, false, false).expect("Save fail");
    let result_data = buffer.into_inner();
    
    assert!(result_data.len() > data.len(), "Saved data should be larger than original");
    
    // 4. Reload and Verify
    let reloaded_doc = ferruginous_sdk::loader::load_document_structure(&result_data).expect("Reload fail");
    
    // Check if new object is present in xref
    assert!(reloaded_doc.xref_index.entries.contains_key(&new_ref.id));
    
    // Resolve new object from reloaded doc
    let resolver = ferruginous_sdk::resolver::PdfResolver {
        data: Box::leak(Box::new(result_data)),
        index: std::sync::Arc::new(reloaded_doc.xref_index),
        security: None,
        cache: std::sync::Mutex::new(std::collections::BTreeMap::new()),
    };
    
    let obj = resolver.resolve(&new_ref).expect("Resolve new obj fail");
    assert_eq!(obj, Object::Integer(999));
}
