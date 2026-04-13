#![allow(clippy::all, missing_docs)]
//! Test module

//! Integration tests for text extraction from PDF content streams.
use ferruginous_sdk::content::{Processor, parse_content_stream};
use ferruginous_sdk::core::{Object, Resolver, PdfResult};
use ferruginous_sdk::font::Font;
use ferruginous_sdk::resources::Resources;
use std::collections::BTreeMap;

struct MockResolver;
impl Resolver for MockResolver {
    fn resolve(&self, _r: &ferruginous_sdk::core::Reference) -> PdfResult<Object> {
        Err(ferruginous_sdk::core::PdfError::ResourceError("Mock resolver".into()))
    }
}

#[test]
fn test_extraction_simple() {
    // 1. Setup mock resources with a simple font
    let mut font_dict = BTreeMap::new();
    font_dict.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    font_dict.insert(b"Subtype".to_vec(), Object::new_name(b"Type1".to_vec()));
    font_dict.insert(b"BaseFont".to_vec(), Object::new_name(b"Helvetica".to_vec()));
    
    let _font = Font::from_dict(&font_dict, &MockResolver).unwrap();
    let mut fonts = BTreeMap::new();
    fonts.insert(b"F1".to_vec(), Object::new_dict(font_dict));
    
    let mut res_dict = BTreeMap::new();
    res_dict.insert(b"Font".to_vec(), Object::new_dict(fonts));
    
    let resources = Resources::new(res_dict.into(), &MockResolver);
    
    // 2. Prepare content stream: BT /F1 12 Tf 10 20 Td (Hello) Tj ET
    let content = b"BT /F1 12 Tf 10 20 Td (Hello) Tj ET";
    let nodes = parse_content_stream(content).expect("Parse error");
    
    // 3. Process with extraction enabled
    let mut processor = Processor::new(Some(resources), None, None);
    processor.enable_text_extraction();
    processor.process_nodes(&nodes).expect("Process error");
    
    // 4. Verify results
    let layer = processor.text_layer.expect("Text layer missing");
    assert_eq!(layer.elements.len(), 1);
    assert_eq!(layer.elements[0].text, "Hello");
    assert_eq!(layer.elements[0].font_size, 12.0);
}

#[test]
fn test_extraction_multi_tj() {
    let mut font_dict = BTreeMap::new();
    font_dict.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    font_dict.insert(b"Subtype".to_vec(), Object::new_name(b"Type1".to_vec()));
    
    let mut fonts = BTreeMap::new();
    fonts.insert(b"F1".to_vec(), Object::new_dict(font_dict));
    let mut res_dict = BTreeMap::new();
    res_dict.insert(b"Font".to_vec(), Object::new_dict(fonts));
    let resources = Resources::new(res_dict.into(), &MockResolver);
    
    // BT /F1 12 Tf (Hello) Tj 0 -15 Td (World) Tj ET
    let content = b"BT /F1 12 Tf (Hello) Tj 0 -15 Td (World) Tj ET";
    let nodes = parse_content_stream(content).unwrap();
    
    let mut processor = Processor::new(Some(resources), None, None);
    processor.enable_text_extraction();
    processor.process_nodes(&nodes).unwrap();
    
    let layer = processor.text_layer.unwrap();
    assert_eq!(layer.elements.len(), 2);
    assert_eq!(layer.elements[0].text, "Hello");
    assert_eq!(layer.elements[1].text, "World");
    assert_eq!(layer.full_text(), "HelloWorld");
}
