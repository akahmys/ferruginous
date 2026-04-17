#![allow(clippy::all, missing_docs)]
//! Test module for vertical writing mode.
#![allow(clippy::all, missing_docs)]

use ferruginous_sdk::text::TextState;
use ferruginous_sdk::font::Font;
use ferruginous_sdk::cmap::CMap;
use kurbo::Affine;
use std::collections::BTreeMap;

#[test]
fn test_vertical_advance() {
    let mut state = TextState::default();
    state.wmode = 1;
    state.font_size = 12.0;
    state.char_spacing = 1.0; // Tc = 1.0
    state.word_spacing = 2.0; // Tw = 2.0
    
    // Standard Japanese character with default vertical metrics (w1y = -1000)
    let glyph_width = 1000.0;
    let v_adv = Some(-1000.0);
    let is_space = false;
    
    let (ty, _before) = state.advance_glyph(is_space, glyph_width, v_adv, 0.0, true);
    
    const EPSILON: f64 = 0.000_1;
    assert!((ty - -13.0).abs() < EPSILON, "Vertical displacement should be -13.0");
    
    // Verify matrix update
    let coeffs = state.matrix.as_coeffs();
    assert!((coeffs[4] - 0.0).abs() < EPSILON);
    assert!((coeffs[5] - -13.0).abs() < EPSILON);
    
    // Test with word spacing
    state.matrix = Affine::IDENTITY;
    let (ty2, _) = state.advance_glyph(true, glyph_width, v_adv, 0.0, true);
    assert!((ty2 - -15.0).abs() < EPSILON);
}

#[test]
fn test_latin_rotation_detection() {
    let mut font = Font::new_dummy_multi_byte().unwrap();
    font.base_font = b"Helvetica".to_vec();
    font.wmode = 1; // Explicit vertical mode
    
    // ASCII 'A'
    assert!(font.char_should_rotate_vertical(b"A"));
    // ASCII '1'
    assert!(font.char_should_rotate_vertical(b"1"));
}

#[test]
fn test_aj1_fallback() {
    let mut font = Font::new_dummy_multi_byte().unwrap();
    font.base_font = b"Adobe-Japan1-Font".to_vec();
    font.encoding_cmap = CMap::new_predefined("Identity-H"); // Identity-H returns CIDs
    font.font_matrix = [0.001, 0.0, 0.0, 0.001, 0.0, 0.0];
    font.wmode = 0;

    // CID 232 is Full-width '0' (U+FF10) in my partial AJ1 map
    let cid_bytes = [0, 232]; 
    let unicode = font.to_unicode_string(&cid_bytes);
    assert_eq!(unicode, "０", "Should use AJ1 fallback when ToUnicode is missing");
}

#[test]
fn test_full_width_space_detection() {
    let font = Font::new_dummy_multi_byte().unwrap();
    // Identity-H mapping for CID 231 (Ideographic Space)
    let space_cid = [0, 231];
    assert!(font.is_space_char(&space_cid), "CID 231 should be recognized as space");
}
