use ferruginous_sdk::text::TextState;
use ferruginous_sdk::font::Font;
use kurbo::Affine;

#[test]
fn test_japanese_text_advancement() {
    let mut state = TextState::default();
    state.font_size = 12.0;
    state.char_spacing = 2.0; // Tc = 2.0 (unscaled)
    state.horizontal_scaling = 100.0; // Th = 1.0
    
    // Simulate a Japanese character (standard 1000-unit width)
    let glyph_width = 1000.0;
    let is_space = false;
    let v_adv = None;
    let tj_adj = 0.0;
    
    let (tx, before) = state.advance_glyph(is_space, glyph_width, v_adv, tj_adj, true);
    
    // Expected tx = ( (1000/1000 * 12.0) + 2.0 ) * 1.0 = 14.0
    assert_eq!(tx, 14.0, "Displacement should include Tc");
    
    // Verify cumulative matrix
    let coeffs = state.matrix.as_coeffs();
    assert_eq!(coeffs[4], 14.0, "Matrix translation should update by tx");
    
    // Test with Horizontal Scaling (Tz = 50%)
    state.matrix = Affine::IDENTITY;
    state.horizontal_scaling = 50.0; // Th = 0.5
    let (tx2, _) = state.advance_glyph(is_space, glyph_width, v_adv, tj_adj, true);
    
    // Expected tx2 = ( (1000/1000 * 12.0) + 2.0 ) * 0.5 = 7.0
    assert_eq!(tx2, 7.0, "Displacement should be scaled by Th");
}

#[test]
fn test_word_spacing_in_identity_h() {
    let mut state = TextState::default();
    state.font_size = 10.0;
    state.word_spacing = 5.0; // Tw = 5.0
    
    // 2-byte space character [0, 32] -> should NOT apply Tw automatically?
    // In horizontal writing, Tw applies only to ASCII 32.
    // My code currently checks `char_code == [32]`.
    
    let glyph_width = 500.0; // Half-width space
    let is_space = true;
    
    let (tx, _) = state.advance_glyph(is_space, glyph_width, None, 0.0, true);
    
    // Expected tx = (500/1000 * 10.0) + 5.0 = 5.0 + 5.0 = 10.0
    assert_eq!(tx, 10.0, "Word spacing should apply if is_space is true");
}
