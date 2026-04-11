use ferruginous_sdk::text_layer::{TextLayer, TextElement};
use ferruginous_sdk::search::{SearchEngine, SearchResult};
use kurbo::Rect;
use ferruginous_sdk::graphics::Color;

#[test]
fn test_search_simple() {
    let mut layer = TextLayer::new();
    layer.add_element(TextElement {
        text: "Hello".to_string(),
        bbox: Rect::new(0.0, 0.0, 50.0, 10.0),
        font_name: b"Helvetica".to_vec(),
        font_size: 12.0,
        matrix: kurbo::Affine::IDENTITY,
        color: Color::Gray(0.0),
    });
    layer.add_element(TextElement {
        text: "World".to_string(),
        bbox: Rect::new(60.0, 0.0, 110.0, 10.0),
        font_name: b"Helvetica".to_vec(),
        font_size: 12.0,
        matrix: kurbo::Affine::IDENTITY,
        color: Color::Gray(0.0),
    });

    let results = SearchEngine::search_literal(&layer, "Hello", true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].matched_text, "Hello");
    assert_eq!(results[0].rects[0], Rect::new(0.0, 0.0, 50.0, 10.0));
}

#[test]
fn test_search_case_insensitive() {
    let mut layer = TextLayer::new();
    layer.add_element(TextElement {
        text: "Hello".to_string(),
        bbox: Rect::new(0.0, 0.0, 50.0, 10.0),
        font_name: b"Helvetica".to_vec(),
        font_size: 12.0,
        matrix: kurbo::Affine::IDENTITY,
        color: Color::Gray(0.0),
    });

    let results = SearchEngine::search_literal(&layer, "hello", false);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].matched_text, "Hello");
}

#[test]
fn test_search_fragmented() {
    let mut layer = TextLayer::new();
    layer.add_element(TextElement {
        text: "Hell".to_string(),
        bbox: Rect::new(0.0, 0.0, 40.0, 10.0),
        font_name: b"Helvetica".to_vec(),
        font_size: 12.0,
        matrix: kurbo::Affine::IDENTITY,
        color: Color::Gray(0.0),
    });
    layer.add_element(TextElement {
        text: "o World".to_string(),
        bbox: Rect::new(40.0, 0.0, 100.0, 10.0),
        font_name: b"Helvetica".to_vec(),
        font_size: 12.0,
        matrix: kurbo::Affine::IDENTITY,
        color: Color::Gray(0.0),
    });

    let results = SearchEngine::search_literal(&layer, "Hello", true);
    assert_eq!(results.len(), 1);
    // Note: SearchResult currently captures whole elements that contribute to the match
    assert_eq!(results[0].rects.len(), 2);
}
