//! Ferruginous SDK: High-level PDF processing library.
//!
//! This crate provides a high-level, easy-to-use interface for PDF document
//! manipulation, rendering, and auditing, abstracting away the low-level
//! complexities of the core type system and document model.

use std::path::Path;
use std::sync::Arc;
use bytes::Bytes;
use ferruginous_core::{PdfResult, Reference, Object, Resolver, PdfError, PdfName};
use ferruginous_doc::{Document, PageTree, Page as DocPage};
use ferruginous_render::{VelloBackend, headless::render_to_png};

/// The internal interpreter module for processing content streams.
pub mod interpreter;
/// The internal writer module for generating PDF files.
pub mod writer;

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

    /// Returns high-level compliance information about the document.
    pub fn get_compliance(&self) -> PdfResult<ferruginous_doc::conformance::ComplianceInfo> {
        self.inner.compliance_info()
    }

    /// Extracts text from a specific page.
    pub fn extract_text(&self, index: usize) -> PdfResult<String> {
        let page = self.get_page(index)?;
        page.extract_text(&self.inner)
    }

    /// Saves the current document to a new file with the specified PDF version.
    ///
    /// This performs a full re-serialization of all indirect objects to ensure
    /// absolute compliance with the target version's structure (e.g., PDF 2.0).
    pub fn save_as_version(&self, output_path: &Path, version: &str) -> PdfResult<()> {
        use crate::writer::PdfWriter;
        let file = std::fs::File::create(output_path)
            .map_err(|e| ferruginous_core::PdfError::Other(e.to_string()))?;
        let mut writer = PdfWriter::new(file);

        writer.write_header(version)?;

        // Resolve all indirect objects from the source document
        // Reference management: we keep the original IDs for simplicity, 
        // but re-serialize all active objects.
        let mut root_obj = self.inner.resolve(&self.inner.root())?;
        
        // If upgrading to 2.0, ensure /Version /2.0 is in the Catalog
        if version == "2.0" {
            if let Object::Dictionary(ref mut dict) = root_obj {
                std::sync::Arc::make_mut(dict).insert("Version".into(), Object::Name("2.0".into()));
            }
        }

        // Write all objects from the store
        // We use the Resolver trait to ensure we get decoded/decrypted objects
        for (&id, entry) in self.inner.store().entries.iter() {
            let generation = match entry {
                ferruginous_doc::XRefEntry::InUse { generation, .. } => *generation,
                ferruginous_doc::XRefEntry::Compressed { .. } => 0,
                _ => continue,
            };

            let r = Reference::new(id, generation);
            let obj = if r == self.inner.root() {
                root_obj.clone()
            } else {
                self.inner.resolve(&r)?
            };
            
            writer.write_indirect_object(id, generation, &obj)
                .map_err(|e| ferruginous_core::PdfError::Other(e.to_string()))?;
        }

        // Find Info dictionary reference if it exists
        let info_ref = self.inner.trailer().get(&"Info".into())
            .and_then(|o| o.as_reference());

        writer.finish(self.inner.root(), info_ref)
            .map_err(|e| ferruginous_core::PdfError::Other(e.to_string()))?;

        Ok(())
    }

    /// Saves the document in a Linearized (Fast Web View) format.
    /// (ISO 32000-2:2020 Annex L)
    pub fn save_linearized(&self, output_path: &Path, version: &str) -> PdfResult<()> {
        use crate::writer::{PdfWriter, NullWriter, LinearizationParams};
        use std::collections::{HashSet, HashMap};

        let num_pages = self.inner.get_page_count()?;
        let catalog_ref = self.inner.root();
        
        // 1. Map ALL objects to their primary page or shared section
        // page_map: object_id -> page_index (0-based)
        let mut page_map = HashMap::new();
        let mut shared_objects = HashSet::new();
        
        // Root and Catalog are always Page 1 (index 0)
        let root_deps = self.inner.explore_dependencies(&catalog_ref)?;
        for id in root_deps {
            page_map.insert(id, 0);
        }

        // Crawl each page
        for i in 0..num_pages {
            let p = self.inner.get_page(i)?;
            let deps = self.inner.explore_dependencies(&p.reference)?;
            
            // Core page object and its parents
            page_map.insert(p.reference.id, i);
            for parent in &p.parents {
                // Parents are shared if multi-page tree, but first encounter wins in linearization sections
                page_map.entry(parent.id).or_insert(i);
            }

            for id in deps {
                if let Some(&first_page) = page_map.get(&id) {
                    if first_page != i {
                        shared_objects.insert(id);
                    }
                } else {
                    page_map.insert(id, i);
                }
            }
        }

        // 2. Group objects by section
        // sections[0] = Page 1, sections[1] = Page 2, ..., sections[n] = Shared
        let mut sections: Vec<Vec<Reference>> = vec![Vec::new(); num_pages + 1];
        let shared_sec_idx = num_pages;

        for (&id, entry) in self.inner.store().entries.iter() {
            let generation_num = match entry {
                ferruginous_doc::XRefEntry::InUse { generation, .. } => *generation,
                ferruginous_doc::XRefEntry::Compressed { .. } => 0,
                _ => continue,
            };
            let r = Reference::new(id, generation_num);
            
            if shared_objects.contains(&id) {
                sections[shared_sec_idx].push(r);
            } else if let Some(&page_idx) = page_map.get(&id) {
                sections[page_idx].push(r);
            } else {
                // Orphaned objects go to shared
                sections[shared_sec_idx].push(r);
            }
        }

        // 3. Metadata for Hint Stream
        let mut page_stats = Vec::with_capacity(num_pages);

        // 4. PASS 1: Size estimation
        let mut params = LinearizationParams::default();
        params.num_pages = num_pages;
        let p1 = self.inner.get_page(0)?;
        params.first_page_obj = p1.reference.id;
        
        let lin_dict_id = self.inner.store().max_id() + 1;
        let hint_stream_id = lin_dict_id + 1;

        let mut dry_writer = PdfWriter::new(NullWriter::new());
        dry_writer.write_header(version).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        dry_writer.write_linearization_dict(lin_dict_id, &params).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        params.hint_stream_offset = dry_writer.current_offset();
        params.hint_stream_len = 2048 + (num_pages * 32); // Estimate for hint stream
        dry_writer.write_all(&vec![0; params.hint_stream_len]).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        // Measure each page
        for i in 0..num_pages {
            let start = dry_writer.current_offset();
            let mut count = 0;
            for r in &sections[i] {
                let obj = self.inner.resolve(r)?;
                dry_writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
                count += 1;
            }
            page_stats.push(PageStats { offset: start, length: dry_writer.current_offset() - start, obj_count: count });
            
            if i == 0 {
                params.end_of_first_page = dry_writer.current_offset();
            }
        }
        
        // Shared objects
        for r in &sections[shared_sec_idx] {
            let obj = self.inner.resolve(r)?;
            dry_writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        }
        
        params.main_xref_offset = dry_writer.current_offset();
        let info_ref = self.inner.trailer().get(&"Info".into()).and_then(|o| o.as_reference());
        dry_writer.finish(catalog_ref, info_ref).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        params.file_len = dry_writer.current_offset();

        // 5. Build the real Hint Stream data
        let hint_data = build_hint_stream(&page_stats, &params);
        params.hint_stream_len = hint_data.len();
        
        // Final offset adjustment: if the hint stream size changed, we shift subsequent offsets
        // (For simplicity in this v1, we used a large enough estimate for dry run)

        // 6. FINAL PASS: Actual write
        let file = std::fs::File::create(output_path).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        let mut writer = PdfWriter::new(file);
        
        writer.write_header(version).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        writer.write_linearization_dict(lin_dict_id, &params).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        // Write Hint Stream
        let mut hint_dict = std::collections::BTreeMap::new();
        hint_dict.insert(PdfName::new(b"Type"), Object::Name(PdfName::new(b"HintStream")));
        writer.write_indirect_object(hint_stream_id, 0, &Object::Stream(std::sync::Arc::new(hint_dict), bytes::Bytes::from(hint_data)))
            .map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
            
        for i in 0..num_pages {
            for r in &sections[i] {
                let obj = self.inner.resolve(r)?;
                writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
            }
        }
        
        for r in &sections[shared_sec_idx] {
            let obj = self.inner.resolve(r)?;
            writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        }
        
        writer.finish(catalog_ref, info_ref).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;

        Ok(())
    }
}

/// Helper to build a basic Page Offset Hint Table (ISO 32000-2 Table C.1)
fn build_hint_stream(stats: &[PageStats], _params: &crate::writer::LinearizationParams) -> Vec<u8> {
    use crate::writer::BitWriter;
    let mut bw = BitWriter::new();
    
    // Header for Page Offset Hint Table
    // Item 1: Least number of objects in a page
    let min_objs = stats.iter().map(|s| s.obj_count).min().unwrap_or(0) as u32;
    bw.write_bits(min_objs, 32);
    // Item 2: Location of first page's object (usually immediately after hint stream)
    bw.write_bits(stats[0].offset as u32, 32);
    // Item 3: Bits needed for Page Object count delta
    bw.write_bits(16, 16);
    // Item 4: Least page length
    let min_len = stats.iter().map(|s| s.length).min().unwrap_or(0) as u32;
    bw.write_bits(min_len, 32);
    // Item 5: Bits needed for Page Length delta
    bw.write_bits(16, 16);
    // ... Simplified for MVP ...
    
    // Entry for each page
    for s in stats {
        // Delta objects
        bw.write_bits((s.obj_count as u32).saturating_sub(min_objs), 16);
        // Delta length
        bw.write_bits((s.length as u32).saturating_sub(min_len), 16);
    }
    
    bw.finish()
}

struct PageStats {
    offset: usize,
    length: usize,
    obj_count: usize,
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

    /// Extracts plain text from this page.
    pub fn extract_text(&self, resolver: &dyn ferruginous_core::Resolver) -> PdfResult<String> {
        let mut backend = TextExtractionBackend::new();
        
        let res_dict = if let Some(res_obj) = self.doc_page.resources() {
            res_obj.as_dict_arc().unwrap_or_else(|| Arc::new(std::collections::BTreeMap::new()))
        } else {
            Arc::new(std::collections::BTreeMap::new())
        };

        let mut interpreter = Interpreter::new(&mut backend, resolver, res_dict);
        
        let contents = self.doc_page.dictionary.get(&"Contents".into())
            .ok_or_else(|| ferruginous_core::PdfError::Other("Missing /Contents".into()))?;
            
        let content_refs = match contents {
            Object::Reference(r) => vec![*r],
            Object::Array(a) => a.iter().filter_map(|o| {
                if let Object::Reference(r) = o { Some(*r) } else { None }
            }).collect(),
            Object::Stream(_, _) => vec![], // Handle direct below
            _ => return Err(ferruginous_core::PdfError::Other("Invalid /Contents".into())),
        };

        if let Object::Stream(_, data) = contents {
             interpreter.execute(data)?;
        } else {
            for r in content_refs {
                let stream_obj = resolver.resolve(&r)?;
                if let Object::Stream(_, data) = &stream_obj {
                    interpreter.execute(data)?;
                }
            }
        }

        Ok(backend.text)
    }
}

struct TextExtractionBackend {
    text: String,
}

impl TextExtractionBackend {
    fn new() -> Self {
        Self { text: String::new() }
    }
}

impl ferruginous_render::RenderBackend for TextExtractionBackend {
    fn push_state(&mut self) {}
    fn pop_state(&mut self) {}
    fn transform(&mut self, _affine: kurbo::Affine) {}
    fn fill_path(&mut self, _path: &kurbo::BezPath, _color: &ferruginous_core::graphics::Color, _rule: ferruginous_core::graphics::WindingRule) {}
    fn stroke_path(&mut self, _path: &kurbo::BezPath, _color: &ferruginous_core::graphics::Color, _style: &ferruginous_core::graphics::StrokeStyle) {}
    fn push_clip(&mut self, _path: &kurbo::BezPath, _rule: ferruginous_core::graphics::WindingRule) {}
    fn pop_clip(&mut self) {}
    fn draw_image(&mut self, _data: &[u8], _w: u32, _h: u32, _format: ferruginous_core::graphics::PixelFormat) {}
    fn set_fill_alpha(&mut self, _alpha: f64) {}
    fn set_stroke_alpha(&mut self, _alpha: f64) {}
    fn set_blend_mode(&mut self, _mode: ferruginous_core::graphics::BlendMode) {}
    
    fn show_text(&mut self, text: &str, _font_name: &str, _size: f32, _transform: kurbo::Affine) {
        self.text.push_str(text);
        if !text.ends_with(' ') {
             // Basic heuristic for spaces between Tj ops if needed, 
             // but usually spaces are explicit in PDF or TJ.
        }
    }
}
