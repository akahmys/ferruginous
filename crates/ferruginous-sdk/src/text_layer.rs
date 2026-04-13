//! Text Layer and Elements for extraction and search.
//!
//! (ISO 32000-2:2020 Clause 9.10 - Extraction of Text Content)

use kurbo::{Rect, Affine};
use crate::graphics::Color;

/// Represents a contiguous block of text on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct TextElement {
    /// The Unicode representation of the text.
    pub text: String,
    /// The bounding box of the text in page (PDF User Space) coordinates.
    pub bbox: Rect,
    /// The font name/reference used for this text.
    pub font_name: Vec<u8>,
    /// The font size in points.
    pub font_size: f64,
    /// The transformation matrix applied to this element (excluding page flip).
    pub matrix: Affine,
    /// The fill color of the text.
    pub color: Color,
}

/// Represents the entire text layer of a single PDF page.
#[derive(Debug, Clone, Default)]
pub struct TextLayer {
    /// The collection of text elements in the order they appear in the content stream.
    pub elements: Vec<TextElement>,
}

impl TextLayer {
    /// Creates a new, empty text layer.
    #[must_use] pub fn new() -> Self {
        Self::default()
    }

    /// Adds a text element to the layer.
    pub fn add_element(&mut self, element: TextElement) {
        self.elements.push(element);
    }

    /// Returns the full text of the page as a single string, normalized for readable extraction.
    #[must_use] pub fn full_text(&self) -> String {
        let mut result = String::new();
        let mut last_bbox: Option<Rect> = None;

        for e in &self.elements {
            if let Some(last) = last_bbox {
                // Heuristic: If there's a significant vertical jump, it's a new line.
                // In PDF User Space, Y usually increases upwards, so a jump is a change in y0/y1.
                let vertical_gap = (e.bbox.y0 - last.y0).abs();
                let horizontal_gap = e.bbox.x0 - last.x1;

                if vertical_gap > 5.0 {
                    // New line.
                    // Japanese: If the last char of current result is CJK and first char of next is CJK, 
                    // we usually don't want a space.
                    let _needs_space = self.needs_space_between_lines(&result, &e.text);
                    result.push('\n');
                } else if horizontal_gap > 2.0 {
                    // Significant horizontal gap on the same line.
                    if !result.ends_with(' ') && !e.text.starts_with(' ') {
                        result.push(' ');
                    }
                }
            }
            result.push_str(&e.text);
            last_bbox = Some(e.bbox);
        }
        result
    }

    fn needs_space_between_lines(&self, prev_text: &str, next_text: &str) -> bool {
        if prev_text.is_empty() || next_text.is_empty() { return true; }
        let _last_char = prev_text.chars().last().unwrap();
        let _next_char = next_text.chars().next().unwrap();

        // If both are CJK, we don't need a space/newline for many Japanese use cases (e.g. copy-paste optimization)
        // However, for generic text extraction, a newline is usually preferred.
        // We'll follow a standard convention: always add newline unless it's a middle-line fragment.
        true 
    }

    /// Performs a simple hit-test to find the text element at the given page coordinates.
    #[must_use] pub fn hit_test(&self, point: kurbo::Point) -> Option<&TextElement> {
        self.elements.iter().find(|e| e.bbox.contains(point))
    }
}
