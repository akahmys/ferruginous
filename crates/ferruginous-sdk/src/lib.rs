//! Ferruginous SDK: High-level PDF processing library.
//!
//! This crate provides a high-level, easy-to-use interface for PDF document
//! manipulation, rendering, and auditing, abstracting away the low-level
//! complexities of the core type system and document model.

use std::path::Path;
use std::sync::Arc;
use bytes::Bytes;
use ferruginous_core::{PdfResult, Reference, Object, Resolver};
use ferruginous_doc::{Document, PageTree, Page as DocPage};
use ferruginous_render::{VelloBackend, headless::render_to_png};

/// The internal interpreter module for processing content streams.
pub mod interpreter;

use crate::interpreter::Interpreter;

/// High-level entry point for interacting with a PDF document.
///
/// This struct provides a simplified interface for common PDF tasks like
/// reading, querying page trees, and rendering content.
pub struct PdfDocument {
    inner: Document,
}

impl PdfDocument {
    /// Opens a PDF document from a byte buffer.
    ///
    /// This performs initial structure validation (header, XRef, and Catalog resolution).
    pub fn open(data: Bytes) -> PdfResult<Self> {
        let inner = Document::open(data)?;
        Ok(Self { inner })
    }

    /// Returns the total number of pages in the document.
    pub fn page_count(&self) -> PdfResult<usize> {
        let root = self.pages_root()?;
        let tree = PageTree::new(root, &self.inner);
        tree.count()
    }

    /// Retrieves a handle to a specific page by its zero-based index.
    pub fn get_page(&self, index: usize) -> PdfResult<Page<'_>> {
        let root = self.pages_root()?;
        let tree = PageTree::new(root, &self.inner);
        let doc_page = tree.page(index)?;
        Ok(Page { doc_page })
    }

    fn pages_root(&self) -> PdfResult<Reference> {
        let catalog = self.inner.resolve(&self.inner.root())?;
        let dict = catalog.as_dict().ok_or_else(|| ferruginous_core::PdfError::Other("Catalog is not a dictionary".into()))?;
        let pages_ref = dict.get(&"Pages".into())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| ferruginous_core::PdfError::Other("Missing /Pages in Catalog".into()))?;
        Ok(pages_ref)
    }

    /// Renders a specific page to a PNG file.
    ///
    /// This is an asynchronous operation that utilizes the Vello GPU-accelerated
    /// backend and the headless rendering pipeline.
    pub async fn render_page_to_file(&self, index: usize, output_path: &Path) -> PdfResult<()> {
        let page = self.get_page(index)?;
        
        // Get inherited page resources
        let res_dict = if let Some(res_obj) = page.doc_page.resources() {
            res_obj.as_dict_arc().unwrap_or_else(|| Arc::new(std::collections::BTreeMap::new()))
        } else {
            Arc::new(std::collections::BTreeMap::new())
        };

        let mut backend = VelloBackend::new();
        let mut interpreter = Interpreter::new(&mut backend, &self.inner, res_dict);
        
        // Resolve contents
        let contents = page.doc_page.dictionary.get(&"Contents".into())
            .ok_or_else(|| ferruginous_core::PdfError::Other("Missing /Contents".into()))?;
            
        let content_refs = match contents {
            Object::Reference(r) => vec![*r],
            Object::Array(a) => a.iter().filter_map(|o| {
                if let Object::Reference(r) = o { Some(*r) } else { None }
            }).collect(),
            Object::Stream(_, _) => {
                // Handle direct stream object (rare but possible)
                if let Object::Reference(r) = contents { vec![*r] } else { vec![] }
            }
            _ => return Err(ferruginous_core::PdfError::Other("Invalid /Contents".into())),
        };

        // If it was already a stream in-place (not a reference)
        if let Object::Stream(_, data) = contents {
             interpreter.execute(data)?;
        } else {
            for r in content_refs {
                let stream_obj = self.inner.resolve(&r)?;
                if let Object::Stream(_, data) = &stream_obj {
                    interpreter.execute(data)?;
                }
            }
        }

        // Get mediabox for dimensions
        let mediabox = page.doc_page.media_box()
            .and_then(|o| o.as_array())
            .ok_or_else(|| ferruginous_core::PdfError::Other("Missing /MediaBox".into()))?;
            
        let w = (mediabox[2].as_f64().ok_or_else(|| ferruginous_core::PdfError::Other("Invalid MediaBox width".into()))? 
                 - mediabox[0].as_f64().ok_or_else(|| ferruginous_core::PdfError::Other("Invalid MediaBox x".into()))?).abs() as u32;
        let h = (mediabox[3].as_f64().ok_or_else(|| ferruginous_core::PdfError::Other("Invalid MediaBox height".into()))? 
                 - mediabox[1].as_f64().ok_or_else(|| ferruginous_core::PdfError::Other("Invalid MediaBox y".into()))?).abs() as u32;

        render_to_png(backend.scene(), w, h, output_path).await
            .map_err(|e| ferruginous_core::PdfError::Other(e.to_string()))?;

        Ok(())
    }
}

/// A handle to a specific page within a [PdfDocument].
pub struct Page<'a> {
    doc_page: DocPage<'a>,
}

impl Page<'_> {
    /// Returns the indirect reference of this page object.
    pub fn reference(&self) -> Reference {
        self.doc_page.reference
    }
}
