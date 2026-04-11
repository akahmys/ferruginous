//! Text Layer and Elements for extraction and search.
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

    /// Returns the full text of the page as a single string, joined by spaces or newlines.
    #[must_use] pub fn full_text(&self) -> String {
        self.elements.iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Performs a simple hit-test to find the text element at the given page coordinates.
    #[must_use] pub fn hit_test(&self, point: kurbo::Point) -> Option<&TextElement> {
        self.elements.iter().find(|e| e.bbox.contains(point))
    }
}
