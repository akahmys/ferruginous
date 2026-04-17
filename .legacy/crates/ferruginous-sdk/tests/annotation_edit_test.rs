#![allow(clippy::all, missing_docs)]
//! Test module

use ferruginous_sdk::core::{Object, Reference};
use ferruginous_sdk::loader::PdfDocument;
use ferruginous_sdk::editor::PdfEditor;
use ferruginous_sdk::xref::{MemoryXRefIndex, XRefEntry};
use std::collections::BTreeMap;

#[test]
fn test_add_text_annotation() {
    // 1. Mock minimal PDF (1 page)
    let obj1 = b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n";
    let obj2 = b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n";
    let obj3 = b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources <<>> >> endobj\n";
    
    let mut index = MemoryXRefIndex::default();
    let offset1 = 0;
    let offset2 = obj1.len() as u64;
    let offset3 = offset2 + obj2.len() as u64;
    
    index.insert(1, XRefEntry::InUse { offset: offset1, generation: 0 });
    index.insert(2, XRefEntry::InUse { offset: offset2, generation: 0 });
    index.insert(3, XRefEntry::InUse { offset: offset3, generation: 0 });
    
    let mut data = Vec::new();
    data.extend_from_slice(obj1);
    data.extend_from_slice(obj2);
    data.extend_from_slice(obj3);
    
    let doc = PdfDocument {
        data: data.clone(),
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

    let mut editor = PdfEditor::new(doc).expect("Failed to create editor");
    let page_ref = Reference { id: 3, generation: 0 };

    // 2. Define a text annotation
    let mut annot_dict = BTreeMap::new();
    annot_dict.insert(b"Type".to_vec(), Object::new_name(b"Annot".to_vec()));
    annot_dict.insert(b"Subtype".to_vec(), Object::new_name(b"Text".to_vec()));
    annot_dict.insert(b"Rect".to_vec(), Object::new_array(vec![
        Object::Integer(100), Object::Integer(100), Object::Integer(200), Object::Integer(200)
    ]));
    annot_dict.insert(b"Contents".to_vec(), Object::new_string(b"Hello from M25!".to_vec()));

    // 3. Add annotation (HDD: This should fail to compile or run)
    let annot_ref = editor.add_annotation(page_ref, annot_dict).expect("Failed to add annotation");

    // 4. Verify annotation added to page
    let updated_page = editor.get_object(&page_ref).expect("Failed to get updated page");
    if let Object::Dictionary(dict) = updated_page {
        let annots = dict.get(b"Annots".as_ref()).expect("Page missing /Annots");
        match annots {
            Object::Array(arr) => {
                assert!(arr.contains(&Object::Reference(annot_ref)));
            },
            Object::Reference(r) => {
                let resolved_annots = editor.get_object(r).expect("Failed to resolve /Annots");
                if let Object::Array(arr) = resolved_annots {
                    assert!(arr.contains(&Object::Reference(annot_ref)));
                } else {
                    panic!("Expected array of annotations");
                }
            },
            _ => panic!("Unexpected /Annots type"),
        }
    } else {
        panic!("Expected page dictionary");
    }

    // 5. Remove annotation
    editor.remove_annotation(page_ref, annot_ref).expect("Failed to remove annotation");

    // 6. Verify annotation removed
    let final_page = editor.get_object(&page_ref).expect("Failed to get final page");
    if let Object::Dictionary(dict) = final_page {
        if let Some(annots) = dict.get(b"Annots".as_ref()) {
            match annots {
                Object::Array(arr) => {
                    assert!(!arr.contains(&Object::Reference(annot_ref)));
                },
                Object::Reference(r) => {
                    let resolved_annots = editor.get_object(r).expect("Failed to resolve /Annots");
                    if let Object::Array(arr) = resolved_annots {
                        assert!(!arr.contains(&Object::Reference(annot_ref)));
                    } else {
                        panic!("Expected array of annotations");
                    }
                },
                _ => panic!("Unexpected /Annots type"),
            }
        }
    } else {
        panic!("Expected page dictionary");
    }
}
