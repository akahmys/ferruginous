#![allow(clippy::all, missing_docs)]
//! Test module

use ferruginous_sdk::core::{Object, Reference};
use ferruginous_sdk::serialize as writer;
use ferruginous_sdk::serialize::object_stream::ObjectStreamBuilder;
use ferruginous_sdk::editor::PdfEditor;
use ferruginous_sdk::loader::PdfDocument;
use ferruginous_sdk::xref::{MemoryXRefIndex, XRefEntry};
use std::collections::BTreeMap;
use std::io::Cursor;

#[test]
fn test_xref_stream_serialization() {
    let mut buf = Vec::new();
    let mut dict = BTreeMap::new();
    dict.insert(b"Type".to_vec(), Object::new_name(b"XRef".to_vec()));
    dict.insert(b"Size".to_vec(), Object::Integer(5));
    dict.insert(b"W".to_vec(), Object::new_array(vec![
        Object::Integer(1), Object::Integer(2), Object::Integer(1)
    ]));
    
    let data = vec![
        0, 0, 0, 0,
        1, 0, 120, 0,
        1, 0, 202, 0,
        2, 0, 4, 0,
        2, 0, 4, 1,
    ];

    let obj = Object::new_stream(dict, data);
    writer::write_indirect_object(&mut buf, 5, 0, &obj).expect("Failed to write XRef stream");
    
    let output = String::from_utf8_lossy(&buf);
    assert!(output.contains("/Type /XRef"));
    assert!(output.contains("stream"));
}

#[test]
fn test_object_stream_serialization() {
    let mut builder = ObjectStreamBuilder::new();
    builder.add_object(1, Object::Integer(42)).unwrap();
    builder.add_object(2, Object::new_name(b"Test".to_vec())).unwrap();
    
    let obj = builder.build().expect("Failed to build ObjStm");
    
    if let Object::Stream(dict, data) = obj {
        assert_eq!(dict.get(b"Type".as_ref()).unwrap(), &Object::new_name(b"ObjStm".to_vec()));
        assert_eq!(dict.get(b"N".as_ref()).unwrap(), &Object::Integer(2));
        
        let data_str = String::from_utf8_lossy(&data);
        // Header contains "1 0 2 <offset_to_2> "
        assert!(data_str.contains("1 0")); 
        assert!(data_str.contains("2 "));
        // Body contains "42 /Test"
        assert!(data_str.contains("42"));
        assert!(data_str.contains("/Test"));
    } else {
        panic!("Expected stream object");
    }
}

#[test]
fn test_deterministic_save() {
    let base_data = b"1 0 obj << /Type /Catalog >> endobj\n";
    let mut index = MemoryXRefIndex::default();
    index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 });
    
    let doc = PdfDocument {
        data: base_data.to_vec(),
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

    let mut editor1 = PdfEditor::new(doc.clone()).expect("Failed to create editor");
    let _ = editor1.create_object(Object::new_string(b"Data".to_vec()));
    let mut out1 = Cursor::new(Vec::new());
    editor1.save_incremental(&mut out1, true, true).unwrap();

    let mut editor2 = PdfEditor::new(doc).expect("Failed to create editor");
    let _ = editor2.create_object(Object::new_string(b"Data".to_vec()));
    let mut out2 = Cursor::new(Vec::new());
    editor2.save_incremental(&mut out2, true, true).unwrap();

    // Output should be byte-identical (Deterministic)
    assert_eq!(out1.into_inner(), out2.into_inner());
}
