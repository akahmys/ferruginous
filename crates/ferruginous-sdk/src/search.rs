//! Text search engine for the PDF text layer.
//! (ISO 32000-2:2020 Clause 9.10 - Extraction of Text Content)

use kurbo::Rect;
use crate::text_layer::TextLayer;

/// Represents a match found by the search engine.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The exact text that matched.
    pub matched_text: String,
    /// The set of bounding boxes covering the match (may be multiple if the match spans elements).
    pub rects: Vec<Rect>,
    /// The index of the first matching element in the TextLayer.
    pub start_element_idx: usize,
}

/// A search engine that handles text matching across possibly fragmented TextElements.
pub struct SearchEngine;

impl SearchEngine {
    /// Searches for a literal string within the given text layer.
    /// Returns a list of all occurrences.
    #[must_use] pub fn search_literal(layer: &TextLayer, query: &str, case_sensitive: bool) -> Vec<SearchResult> {
        if query.is_empty() { return Vec::new(); }

        let q = if case_sensitive { query.to_string() } else { query.to_lowercase() };
        let mut results = Vec::new();

        // Simple approach: Join all text and find offsets, then map back to elements.
        // For fragmented text, this is more robust than element-by-element matching.
        let full_text = layer.full_text();
        let searchable_text = if case_sensitive { full_text.clone() } else { full_text.to_lowercase() };

        let mut start_pos = 0;
        while let Some(pos) = searchable_text[start_pos..].find(&q) {
            let actual_pos = start_pos + pos;
            if let Some(res) = Self::map_offset_to_result(layer, actual_pos, q.len(), &full_text) {
                results.push(res);
            }
            start_pos = actual_pos + 1;
            
            // Safety: Avoid infinite loop
            if start_pos >= searchable_text.len() { break; }
        }

        results
    }

    fn map_offset_to_result(layer: &TextLayer, offset: usize, len: usize, _full_text: &str) -> Option<SearchResult> {
        let mut current_offset = 0;
        let mut start_element_idx = None;
        let mut rects = Vec::new();
        let mut matched_text = String::new();
        let end_offset = offset + len;

        for (idx, element) in layer.elements.iter().enumerate() {
            let element_len = element.text.len();
            let element_end = current_offset + element_len;

            // Check if this element contains part of the match
            if element_end > offset && current_offset < end_offset {
                if start_element_idx.is_none() {
                    start_element_idx = Some(idx);
                }
                
                // Calculate which part of the element matches
                // For now, we take the whole element's BBox if it contributes to the match.
                // In a more refined version, we would calculate sub-rects for partial matches.
                rects.push(element.bbox);
                matched_text.push_str(&element.text);
            }

            if current_offset >= end_offset { break; }
            
            // Accurately map offset now that TextLayer::full_text() no longer adds spaces
            current_offset += element_len;
        }

        start_element_idx.map(|idx| SearchResult {
            matched_text,
            rects,
            start_element_idx: idx,
        })
    }
}
