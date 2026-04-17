#![allow(clippy::all, missing_docs)]
//! Test module for graphics extraction.
#![allow(clippy::float_cmp, missing_docs)]

use ferruginous_sdk::core::{Object, Reference, Resolver};
use ferruginous_sdk::loader::{PdfDocument, load_document_structure};
use ferruginous_sdk::page::Page;
use ferruginous_sdk::xref::{MemoryXRefIndex, XRefEntry};
use ferruginous_sdk::graphics::DrawOp;
use std::collections::BTreeMap;
use std::sync::Arc;

#[test]
fn test_graphics_state_extraction() {
    let content_data = b"1.0 0.0 0.0 1.0 100 200 cm 1 0 0 rg 5 w 10 10 m 20 20 l S";
    
    let mut data = Vec::new();
    data.extend(b"1 0 obj << /Type /Page /Contents 2 0 R >> endobj\n");
    let c_offset = data.len();
    data.extend(format!("2 0 obj << /Length {} >> stream\n", content_data.len()).as_bytes());
    data.extend(content_data);
    data.extend(b"\nendstream\nendobj\n");
    
    let mut index = MemoryXRefIndex::default();
    index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 });
    index.insert(2, XRefEntry::InUse { offset: c_offset as u64, generation: 0 });

    let doc = PdfDocument {
        data,
        xref_index: index,
        last_trailer: ferruginous_sdk::trailer::TrailerInfo { 
            last_xref_offset: 0, 
            trailer_dict: std::sync::Arc::new(BTreeMap::new()) 
        },
        security: None,
    };

    let resolver = ferruginous_sdk::resolver::PdfResolver {
        data: &doc.data,
        index: Arc::new(doc.xref_index.clone()),
        security: None,
        cache: std::sync::Mutex::new(BTreeMap::new()),
    };

    let mut page_dict = BTreeMap::new();
    page_dict.insert(b"Type".to_vec(), Object::new_name(b"Page".to_vec()));
    page_dict.insert(b"Contents".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
    let page_dict = std::sync::Arc::new(page_dict);

    let page = Page {
        dictionary: std::sync::Arc::clone(&page_dict),
        reference: Reference { id: 1, generation: 0 },
        resolver: &resolver,
    };

    let state = page.get_state().expect("Failed to extract state");
    
    let c = state.graphics.ctm.as_coeffs();
    assert_eq!(c[4], 100.0);
    assert_eq!(c[5], 200.0);
}

#[test]
fn test_text_bbox_extraction() {
    // 1. Prepare a mock page with Font and Tj
    // Translation (100, 200) -> BT -> Font F1 size 12 -> Show "A" at (0,0) rel to Tm
    // Matrix Tm: [1 0 0 1 50 50]
    let content_data = b"1 0 0 1 100 200 cm BT /F1 12 Tf 1 0 0 1 50 50 Tm (A) Tj ET";
    
    let mut data = Vec::new();
    // Obj 1: Page
    data.extend(b"1 0 obj << /Type /Page /Contents 2 0 R /Resources << /Font << /F1 3 0 R >> >> >> endobj\n");
    // Obj 2: Contents
    let c_offset = data.len();
    data.extend(format!("2 0 obj << /Length {} >> stream\n", content_data.len()).as_bytes());
    data.extend(content_data);
    data.extend(b"\nendstream\nendobj\n");
    // Obj 3: Font
    let f_offset = data.len();
    data.extend(b"3 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 65 /LastChar 65 /Widths [600] >> endobj\n");

    let mut index = MemoryXRefIndex::default();
    index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 });
    index.insert(2, XRefEntry::InUse { offset: c_offset as u64, generation: 0 });
    index.insert(3, XRefEntry::InUse { offset: f_offset as u64, generation: 0 });

    let doc = PdfDocument {
        data,
        xref_index: index,
        last_trailer: ferruginous_sdk::trailer::TrailerInfo { 
            last_xref_offset: 0, 
            trailer_dict: std::sync::Arc::new(BTreeMap::new()) 
        },
        security: None,
    };

    let resolver = ferruginous_sdk::resolver::PdfResolver {
        data: &doc.data,
        index: Arc::new(doc.xref_index.clone()),
        security: None,
        cache: std::sync::Mutex::new(BTreeMap::new()),
    };

    let page_dict = match resolver.resolve(&Reference { id: 1, generation: 0 }).unwrap() {
        Object::Dictionary(d) => std::sync::Arc::clone(&d),
        _ => panic!("Expected dictionary"),
    };

    let page = Page {
        dictionary: page_dict,
        reference: Reference { id: 1, generation: 0 },
        resolver: &resolver,
    };
    let display_list = page.get_display_list().expect("Failed to get display list");

    // Look for DrawText operation
    let mut found_text = false;
    for op in display_list {
        if let DrawOp::DrawText { glyphs, .. } = op.op {
            found_text = true;
            assert_eq!(glyphs.len(), 1);
            let g = &glyphs[0];
            assert_eq!(g.char_code, b"A");
            
            // Verification of BBox
            // CTM: [1 0 0 1 100 200]
            // Tm: [1 0 0 1 50 50]
            // Trm: CTM * Tm * Scale(12) * Scale(0.001)
            // Baseline at (0,0) in Tm space. 
            // In page space: CTM * Tm * (0,0) = [1 0 0 1 150 250] * (0,0) = (150, 250)
            
            // Glyph BBox for 'A' (Helvetica, 1000 units): [0, -200, 600, 800] (fallback)
            // Width: 600. Ascent: 800. Descent: -200.
            // Scale by 12/1000 = 0.012
            // Expected Height: 1000 * 0.012 = 12.0
            // Expected Width: 600 * 0.012 = 7.2
            
            let rect = g.bbox;
            // X: 150 + 0 * 0.012 = 150
            // Y: 250 + (-200) * 0.012 = 250 - 2.4 = 247.6
            // X1: 150 + 7.2 = 157.2
            // Y1: 250 + 9.6 = 259.6
            
            assert!((rect.x0 - 150.0).abs() < 0.1);
            assert!((rect.y0 - 247.6).abs() < 0.1);
            assert!((rect.width() - 7.2).abs() < 0.1);
            assert!((rect.height() - 12.0).abs() < 0.1);
        }
    }
    assert!(found_text, "DrawText operation not found");
}
