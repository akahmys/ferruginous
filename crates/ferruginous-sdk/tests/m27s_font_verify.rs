#![allow(clippy::all, missing_docs)]
//! Test module

//! Verification tests for M27S font advancement and CMap resolution.

use ferruginous_sdk::core::{Object, Reference, Resolver, PdfError, PdfResult};
use ferruginous_sdk::font::Font;
use ferruginous_sdk::resources::Resources;
use ferruginous_sdk::content::{Processor, parse_content_stream};
use ferruginous_sdk::graphics::DrawOp;
use std::collections::BTreeMap;

struct MockResolver {
    objects: BTreeMap<Reference, Object>,
}

impl Resolver for MockResolver {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        self.objects.get(reference).cloned().ok_or_else(|| {
            PdfError::InvalidType { expected: "Dictionary".into(), found: format!("Ref not found: {reference:?}") }
        })
    }
}

#[test]
fn test_m27s_type0_identity_h_advancement() {
    let mut objects = BTreeMap::new();
    
    // 1. Define Descendant CIDFont
    let mut df_dict = BTreeMap::new();
    df_dict.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    df_dict.insert(b"Subtype".to_vec(), Object::new_name(b"CIDFontType2".to_vec()));
    df_dict.insert(b"BaseFont".to_vec(), Object::new_name(b"MockCIDFont".to_vec()));
    df_dict.insert(b"DW".to_vec(), Object::Integer(1000));
    
    // Widths: CID 1 has width 500, CID 2 has width 800
    // Format: [start [w1 w2 ...]]
    let w_arr = vec![
        Object::Integer(1),
        Object::new_array(vec![Object::Integer(500), Object::Integer(800)])
    ];
    df_dict.insert(b"W".to_vec(), Object::new_array(w_arr));
    
    let df_ref = Reference::new(1, 0);
    objects.insert(df_ref, Object::new_dict(df_dict));
    
    // 2. Define Type 0 Font
    let mut t0_dict = BTreeMap::new();
    t0_dict.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    t0_dict.insert(b"Subtype".to_vec(), Object::new_name(b"Type0".to_vec()));
    t0_dict.insert(b"BaseFont".to_vec(), Object::new_name(b"MockType0".to_vec()));
    t0_dict.insert(b"Encoding".to_vec(), Object::new_name(b"Identity-H".to_vec()));
    t0_dict.insert(b"DescendantFonts".to_vec(), Object::new_array(vec![Object::Reference(df_ref)]));
    
    let resolver = MockResolver { objects };
    
    // 3. Resolve Font
    let font = Font::from_dict(&t0_dict, &resolver).expect("Failed to parse Type 0 font");
    
    assert_eq!(font.subtype, b"Type0");
    assert!(font.encoding_cmap.is_some());
    assert_eq!(font.encoding_cmap.as_ref().unwrap().name, "Identity-H");
    
    // Check width inheritance
    const EPSILON: f64 = 0.000_1;
    assert!((font.cid_width(1) - 500.0).abs() < EPSILON);
    assert!((font.cid_width(2) - 800.0).abs() < EPSILON);
    assert!((font.cid_width(3) - 1000.0).abs() < EPSILON); // Default width DW
    
    // 4. Test Content Stream Processing
    let mut font_resources = BTreeMap::new();
    font_resources.insert(b"F1".to_vec(), Object::new_dict(t0_dict));
    
    let mut res_dict = BTreeMap::new();
    res_dict.insert(b"Font".to_vec(), Object::new_dict(font_resources));
    let resources = Resources::new(res_dict.into(), &resolver);
    
    let mut processor = Processor::new(Some(resources), None, None);
    
    // BT /F1 12 Tf <00010002> Tj ET
    let content = b"BT /F1 12 Tf <00010002> Tj ET";
    let nodes = parse_content_stream(content).expect("Failed to parse content stream");
    processor.process_nodes(&nodes).expect("Failed to process nodes");
    
    // Verify results in display list
    let ops = &processor.display_list;
    let draw_text = ops.iter().find(|op| matches!(op.op, DrawOp::DrawText { .. }));
    
    if let Some(cmd) = draw_text {
        if let DrawOp::DrawText { glyphs, size, .. } = &cmd.op {
            assert!((*size - 12.0).abs() < EPSILON);
            assert_eq!(glyphs.len(), 2);
            
            // CID 1 (00 01)
            assert_eq!(glyphs[0].char_code, vec![0, 1]);
            assert!((glyphs[0].x_advance - 500.0).abs() < EPSILON);
            
            // CID 2 (00 02)
            assert_eq!(glyphs[1].char_code, vec![0, 2]);
            assert!((glyphs[1].x_advance - 800.0).abs() < EPSILON);
        } else {
            panic!("DrawText operation not found");
        }
    } else {
        panic!("DrawText operation not found");
    }
}
