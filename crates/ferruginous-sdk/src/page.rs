//! Page Tree and Page object management.
//! (ISO 32000-2:2020 Clause 7.7.3)

use crate::core::{Object, Reference, Resolver, PdfError, PdfResult, ContentErrorVariant};
use crate::arlington::ArlingtonModel;
use crate::resources::Resources;
use crate::annotation::Annotation;
use crate::graphics::DrawCommand;
use crate::text_layer::TextLayer;
use std::collections::BTreeMap;
use std::path::Path;

/// Combined graphics and text state extracted from a page's content stream.
/// (Clause 7.8)
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedState {
    /// The final graphics state (CTM, colors, paths).
    pub graphics: crate::graphics::GraphicsState,
    /// The final text state (font, matrix, spacing).
    pub text: crate::text::TextState,
}

/// Represents a single page in a PDF document.
/// (ISO 32000-2:2020 Clause 7.7.3.3)
pub struct Page<'a> {
    /// The local page dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The object reference of this page.
    pub reference: Reference,
    /// The resolver instance for object lookups.
    pub resolver: &'a dyn Resolver,
}

impl<'a> Page<'a> {
    /// Retrieves an attribute from the page dictionary, resolving inheritance
    /// from parent nodes if necessary (Clause 7.7.3.4).
    #[must_use] pub fn inherited_attribute(&self, key: &[u8]) -> Option<Object> {
        assert!(!key.is_empty());
        
        // Start with the local dictionary
        if let Some(obj) = self.dictionary.get(key) {
            return Some(obj.clone());
        }

        // Otherwise, move up the parent chain
        let mut current_parent = self.dictionary.get(b"Parent".as_ref()).and_then(|obj| {
            if let Object::Reference(r) = obj { Some(*r) } else { None }
        });

        // RR-10 v2 Rule 6: Skip recursion, set hard limit on iterations
        for _ in 0..32 {
            if let Some(parent_ref) = current_parent {
                if let Ok(Object::Dictionary(parent_dict)) = self.resolver.resolve(&parent_ref) {
                    if let Some(obj) = parent_dict.get(key) {
                        return Some(obj.clone());
                    }
                    // Update parent_ref for the next level
                    current_parent = parent_dict.get(b"Parent".as_ref()).and_then(|obj| {
                        if let Object::Reference(r) = obj { Some(*r) } else { None }
                    });
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        None
    }

    /// Retrieves the `MediaBox` for the page, resolving inheritance if necessary.
    #[must_use] pub fn media_box(&self) -> Option<Vec<f64>> {
        self.inherited_attribute(b"MediaBox").and_then(|obj| {
            if let Object::Array(arr) = obj {
                let mut coords = Vec::with_capacity(4);
                for item in arr.iter() {
                    match item {
                        Object::Integer(i) => coords.push(*i as f64),
                        Object::Real(f) => coords.push(*f),
                        _ => {}
                    }
                }
                if coords.len() == 4 { Some(coords) } else { None }
            } else {
                None
            }
        })
    }

    /// Retrieves the `MediaBox` for the page as a fixed-size array.
    #[must_use] pub fn media_box_array(&self) -> Option<[f64; 4]> {
        self.media_box().and_then(|v| {
            if v.len() == 4 { Some([v[0], v[1], v[2], v[3]]) } else { None }
        })
    }

    /// Retrieves the Resources for the page, resolving inheritance if necessary.
    #[must_use] pub fn resources(&self) -> Option<Resources<'a>> {
        self.inherited_attribute(b"Resources").and_then(|obj| {
            match obj {
                Object::Dictionary(dict) => Some(Resources::new(std::sync::Arc::clone(&dict), self.resolver)),
                Object::Reference(r) => {
                    if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(&r) {
                        Some(Resources::new(dict, self.resolver))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }

    /// Retrieves the list of annotations for the page (Clause 12.5).
    #[must_use] pub fn get_annotations(&self) -> Vec<Annotation<'a>> {
        let mut annots = Vec::new();
        
        let annots_obj = if let Some(o) = self.dictionary.get(b"Annots".as_ref()) {
            o.clone()
        } else {
            return annots;
        };

        let annots_arr = match annots_obj {
            Object::Array(arr) => std::sync::Arc::clone(&arr),
            Object::Reference(r) => {
                if let Ok(Object::Array(arr)) = self.resolver.resolve(&r) {
                    arr
                } else {
                    return annots;
                }
            }
            _ => return annots,
        };

        // RR-10 v2 Rule 9: Set a pragmatic limit on iterations
        for item in annots_arr.iter().take(255) {
            match item {
                Object::Dictionary(dict) => {
                    // Use a dummy reference if it's an inline dictionary (though usually references)
                    let dummy_ref = Reference { id: 0, generation: 0 };
                    annots.push(Annotation::new(std::sync::Arc::clone(dict), dummy_ref, self.resolver));
                }
                Object::Reference(r) => {
                    if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(r) {
                        annots.push(Annotation::new(dict, *r, self.resolver));
                    }
                }
                _ => {}
            }
        }

        annots
    }

    /// Validates the page object against the Arlington PDF Model.
    pub fn validate<P: AsRef<Path>>(&self, tsv_path: P) -> PdfResult<()> {
        let model = ArlingtonModel::from_tsv(tsv_path)
            .map_err(|e| PdfError::ResourceError(format!("Failed to load Arlington model: {e}")))?;
        assert!(!self.dictionary.is_empty());
        model.validate(&self.dictionary, self.resolver, 2.0, None)
    }

    /// Processes the page's content stream and returns the final graphics and text state.
    /// (ISO 32000-2:2020 Clause 7.8)
    pub fn get_state(&self) -> PdfResult<ExtractedState> {
        let data = self.get_combined_content_data()?;
        let resources = self.resources();
        let mut processor = crate::content::Processor::new(resources, self.media_box_array(), None);
        
        let nodes = crate::content::parse_content_stream(&data)?;
        processor.process_nodes(&nodes)?;

        Ok(ExtractedState {
            graphics: processor.gs_stack.current()?.clone(),
            text: processor.text_state.clone(),
        })
    }

    /// Processes the page's content stream and returns a sequence of drawing operations.
    /// (ISO 32000-2:2020 Clause 7.8)
    pub fn get_display_list(&self) -> PdfResult<Vec<DrawCommand>> {
        let data = self.get_combined_content_data()?;
        let resources = self.resources();
        let mut processor = crate::content::Processor::new(resources, self.media_box_array(), None);
        
        let nodes = crate::content::parse_content_stream(&data)?;
        processor.process_nodes(&nodes)?;

        Ok(processor.display_list)
    }

    /// Processes the page's content stream and returns the extracted text layer.
    /// (ISO 32000-2:2020 Clause 9.10)
    pub fn get_text_layer(&self) -> PdfResult<TextLayer> {
        let data = self.get_combined_content_data()?;
        let resources = self.resources();
        let mut processor = crate::content::Processor::new(resources, self.media_box_array(), None);
        processor.enable_text_extraction();
        
        let nodes = crate::content::parse_content_stream(&data)?;
        processor.process_nodes(&nodes)?;

        Ok(processor.text_layer.unwrap_or_default())
    }

    /// Returns the combined content stream data for the page.
    pub fn get_combined_content_data(&self) -> PdfResult<Vec<u8>> {
        let contents = self.dictionary.get(b"Contents".as_ref())
            .ok_or_else(|| PdfError::ContentError("Page missing /Contents".into()))?;
        
        let mut streams = Vec::new();
        match contents {
            Object::Reference(r) => streams.push(*r),
            Object::Array(arr) => {
                for item in arr.iter() {
                    if let Object::Reference(r) = item { streams.push(*r); }
                }
            }
            _ => return Err(PdfError::ContentError(ContentErrorVariant::UnsupportedType("Unsupported /Contents type".to_string()))),
        }

        let mut all_data = Vec::new();
        for r in &streams {
            let obj = self.resolver.resolve(r)?;
            if let Object::Stream(dict, data) = obj {
                let decoded = crate::filter::decode_stream(&dict, &data)?;
                all_data.extend(decoded);
                all_data.push(b' '); // Ensure separation between streams
            }
        }
        Ok(all_data)
    }
}

/// Represents the logical page tree of the PDF (Clause 7.7.3).
pub struct PageTree<'a> {
    /// Reference to the root /Pages node.
    pub root_pages: Reference,
    /// The resolver instance for object lookups.
    pub resolver: &'a dyn Resolver,
}

impl<'a> PageTree<'a> {
    /// Returns the total number of pages in the document.
    /// (ISO 32000-2:2020 Clause 7.7.3.2)
    pub fn get_count(&self) -> usize {
        if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(&self.root_pages) {
            if let Some(Object::Integer(count)) = dict.get(b"Count".as_ref()) {
                if *count >= 0 {
                    return *count as usize;
                }
            }
        }
        0
    }

    /// Resolves the page at a given index (0-based) using a non-recursive walk.
    pub fn get_page(&self, target_index: usize) -> PdfResult<Page<'a>> {
        let mut stack = vec![(self.root_pages, 0)];
        let mut pages_seen = 0;

        while let Some((node_ref, _)) = stack.last().copied() {
            let node_obj = self.resolver.resolve(&node_ref)?;
            let dict = if let Object::Dictionary(d) = node_obj { d } 
                       else { return Err(PdfError::InvalidType { expected: "Dictionary (Page/Pages)".into(), found: format!("{node_obj:?}") }); };

            let type_name = dict.get(b"Type".as_ref()).and_then(|o| o.as_str());
            
            if type_name == Some(b"Page") {
                if pages_seen == target_index {
                    return Ok(Page { dictionary: dict, reference: node_ref, resolver: self.resolver });
                }
                pages_seen += 1;
                stack.pop();
            } else if type_name == Some(b"Pages") {
                self.process_pages_node(&mut stack, &dict)?;
            } else {
                return Err(PdfError::InvalidType { expected: "/Page or /Pages".into(), found: format!("{type_name:?}") });
            }

            if stack.len() > 32 { return Err(PdfError::ResourceError("Page tree depth limit exceeded".into())); }
        }
        Err(PdfError::ResourceError(format!("Page index {target_index} out of range")))
    }

    fn process_pages_node(&self, stack: &mut Vec<(Reference, usize)>, dict: &std::sync::Arc<BTreeMap<Vec<u8>, Object>>) -> PdfResult<()> {
        let kids = if let Some(Object::Array(k)) = dict.get(b"Kids".as_ref()) { k }
                    else { return Err(PdfError::InvalidType { expected: "Array (/Kids)".into(), found: "Missing or invalid".into() }); };

        let last = stack.last_mut().ok_or_else(|| PdfError::ResourceError("Page tree stack empty unexpectedly".into()))?;
        let child_idx = last.1;
        if child_idx < kids.len() {
            last.1 += 1;
            if let Object::Reference(r) = &kids[child_idx] {
                stack.push((*r, 0));
            }
        } else {
            stack.pop();
        }
        Ok(())
    }

    /// Finds the index of a page given its object reference.
    /// (ISO 32000-2:2020 Clause 7.7.3.3)
    #[must_use] pub fn find_page_index(&self, page_ref: &Reference) -> Option<usize> {
        let count = self.get_count();
        for i in 0..count {
            if let Ok(page) = self.get_page(i) {
                if page.reference == *page_ref {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Validates a specific node in the page tree.
    pub fn validate_node<P: AsRef<Path>>(&self, node_ref: &Reference, tsv_path: P) -> PdfResult<()> {
        let node_obj = self.resolver.resolve(node_ref)?;
        let dict = if let Object::Dictionary(d) = node_obj { d } 
                   else { return Err(PdfError::InvalidType { expected: "Dictionary".into(), found: format!("{node_obj:?}") }); };

        let model = ArlingtonModel::from_tsv(tsv_path)
            .map_err(|e| PdfError::ResourceError(format!("Failed to load Arlington model: {e}")))?;
        
        assert!(!dict.is_empty());
        model.validate(&dict, self.resolver, 2.0, None)
    }
}
