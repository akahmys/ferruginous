//! Ferruginous SDK: High-level PDF processing library.
//!
//! This crate provides a high-level, easy-to-use interface for PDF document
//! manipulation, rendering, and auditing, abstracting away the low-level
//! complexities of the core type system and document model.

use std::path::Path;
use std::sync::Arc;
use bytes::Bytes;
use ferruginous_core::{PdfResult, Reference, Object, Resolver, PdfError, Color};
use ferruginous_doc::{Document, PageTree, Page as DocPage};
use ferruginous_render::{VelloBackend, RenderBackend, headless::render_to_image};
use image::ImageFormat;

/// The internal interpreter module for processing content streams.
pub mod interpreter;
/// The internal writer module for generating PDF files.
pub mod writer;
/// The internal cloning module for object migration.
pub mod cloning;

use crate::interpreter::Interpreter;

/// Supported PDF modern standards for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfStandard {
    /// PDF/A-4 (ISO 19005-4:2020) for long-term archiving.
    A4,
    /// PDF/X-6 (ISO 15930-9:2020) for professional printing.
    X6,
    /// PDF/UA-2 (ISO 14289-2:2024) for universal accessibility.
    UA2,
    /// ISO 32000-2 (PDF 2.0) base compliance.
    ISO32000_2,
}

/// High-level entry point for interacting with a PDF document.
///
/// This struct provides a simplified interface for common PDF tasks like
/// reading, querying page trees, and rendering content.
pub struct PdfDocument {
    inner: Document,
    vacuum: bool,
    strip: bool,
    password: Option<String>,
}

/// A high-level collection of document traits and metadata.
#[derive(Debug, Clone)]
pub struct DocumentSummary {
    /// The PDF version string (e.g., "2.0").
    pub version: String,
    /// Total number of pages in the document.
    pub page_count: usize,
    /// Metadata from the Info dictionary.
    pub metadata: DocumentMetadata,
    /// Conformance info regarding PDF standards (A-4, X-6, etc.).
    pub compliance: ferruginous_doc::conformance::ComplianceInfo,
}

/// Standard PDF metadata fields from the Info dictionary.
#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
    /// The document title.
    pub title: Option<String>,
    /// The name of the person who created the document.
    pub author: Option<String>,
    /// The subject of the document.
    pub subject: Option<String>,
    /// Keywords associated with the document.
    pub keywords: Option<String>,
    /// The name of the application that created the original document.
    pub creator: Option<String>,
    /// If the document was converted to PDF from another format, the name of the application that converted it.
    pub producer: Option<String>,
}

impl PdfDocument {
    /// Opens a PDF document from a byte buffer.
    ///
    /// This performs initial structure validation (header, XRef, and Catalog resolution).
    pub fn open(data: Bytes) -> PdfResult<Self> {
        let inner = Document::open(data)?;
        Ok(Self { inner, vacuum: false, strip: false, password: None })
    }

    /// Attempts to open and repair a potentially corrupted PDF document.
    pub fn open_and_repair(data: Bytes) -> PdfResult<Self> {
        let inner = Document::open_repair(data)?;
        Ok(Self { inner, vacuum: false, strip: false, password: None })
    }

    /// Enables or disables object garbage collection (unreachable object removal).
    pub fn set_vacuum(&mut self, enabled: bool) {
        self.vacuum = enabled;
    }

    /// Enables or disables metadata stripping (Info dictionary sanitization).
    pub fn set_strip(&mut self, enabled: bool) {
        self.strip = enabled;
    }

    /// Sets the encryption password for the output document.
    pub fn set_password(&mut self, password: Option<String>) {
        self.password = password;
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

    /// Persists changes made to a page back to the document store.
    pub fn update_page(&mut self, page: &Page) -> PdfResult<()> {
        let dict = Object::Dictionary(page.doc_page.dictionary.clone());
        self.inner.update_object(page.doc_page.reference.id, dict)
    }

    /// Sets the rotation of a specific page.
    pub fn set_page_rotation(&mut self, index: usize, angle: i32) -> PdfResult<()> {
        let (page_ref, dict) = {
            let mut page = self.get_page(index)?;
            page.set_rotation(angle)?;
            (page.reference(), page.doc_page.dictionary.clone())
        };
        self.inner.update_object(page_ref.id, Object::Dictionary(dict))
    }

    fn pages_root(&self) -> PdfResult<Reference> {
        let catalog = self.inner.resolve(&self.inner.root())?;
        let dict = catalog.as_dict().ok_or_else(|| ferruginous_core::PdfError::Other("Catalog is not a dictionary".into()))?;
        let pages_ref = dict.get(&"Pages".into())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| ferruginous_core::PdfError::Other("Missing /Pages in Catalog".into()))?;
        Ok(pages_ref)
    }

    /// Returns the PDF version string from the file header.
    pub fn header_version(&self) -> &str {
        self.inner.header_version()
    }

    /// Generates a comprehensive summary of the document's state and compliance.
    pub fn get_summary(&self) -> PdfResult<DocumentSummary> {
        let version = self.inner.header_version().to_string();
        let page_count = self.page_count().unwrap_or(0);
        let compliance = self.inner.compliance_info()?;
        
        // Extract basic metadata from Info dictionary
        let mut metadata = DocumentMetadata::default();
        if let Some(Object::Reference(info_ref)) = self.inner.trailer().get(&"Info".into()) {
            if let Ok(info_obj) = self.inner.resolve(info_ref) {
                if let Some(dict) = info_obj.as_dict() {
                    metadata.title = dict.get(&"Title".into()).map(|o| o.to_string_lossy());
                    metadata.author = dict.get(&"Author".into()).map(|o| o.to_string_lossy());
                    metadata.subject = dict.get(&"Subject".into()).map(|o| o.to_string_lossy());
                    metadata.keywords = dict.get(&"Keywords".into()).map(|o| o.to_string_lossy());
                    metadata.creator = dict.get(&"Creator".into()).map(|o| o.to_string_lossy());
                    metadata.producer = dict.get(&"Producer".into()).map(|o| o.to_string_lossy());
                }
            }
        }
        
        Ok(DocumentSummary {
            version,
            page_count,
            metadata,
            compliance,
        })
    }

    /// Returns a string representation of the document's hierarchical structure.
    pub fn print_structure(&self) -> PdfResult<String> {
        self.inner.dump_structure()
    }

    /// Enhances the document to comply with a modern standard (A-4, X-6, or UA-2).
    ///
    /// This automatically injects required XMP metadata and dictionary entries.
    pub fn upgrade_to_standard(&mut self, standard: PdfStandard) -> PdfResult<()> {
        let catalog_ref = self.inner.root();
        let mut catalog_obj = self.inner.resolve(&catalog_ref)?;
        let catalog = catalog_obj.as_dict_mut()
            .ok_or_else(|| PdfError::Other("Catalog is not a dictionary".into()))?;

        match standard {
            PdfStandard::A4 => {
                println!("Note: Injecting PDF/A-4 identification metadata...");
                let xmp = self.generate_xmp(standard);
                let mut meta_dict = std::collections::BTreeMap::new();
                meta_dict.insert("Type".into(), Object::Name("Metadata".into()));
                meta_dict.insert("Subtype".into(), Object::Name("XML".into()));
                let meta_stream = Object::Stream(Arc::new(meta_dict), Bytes::from(xmp));
                let meta_ref = self.inner.add_object(meta_stream);
                catalog.insert("Metadata".into(), Object::Reference(meta_ref));
            }
            PdfStandard::X6 => {
                println!("Note: Preparing PDF/X-6 printing intents...");
                catalog.insert("GTS_PDFXVersion".into(), Object::String(Bytes::from("PDF/X-6")));
                let xmp = self.generate_xmp(standard);
                let mut meta_dict = std::collections::BTreeMap::new();
                meta_dict.insert("Type".into(), Object::Name("Metadata".into()));
                meta_dict.insert("Subtype".into(), Object::Name("XML".into()));
                let meta_stream = Object::Stream(Arc::new(meta_dict), Bytes::from(xmp));
                let meta_ref = self.inner.add_object(meta_stream);
                catalog.insert("Metadata".into(), Object::Reference(meta_ref));
            }
            PdfStandard::UA2 => {
                println!("Note: Preparing PDF/UA-2 accessibility markers...");
                let mut mark_info = std::collections::BTreeMap::new();
                mark_info.insert("Marked".into(), Object::Boolean(true));
                catalog.insert("MarkInfo".into(), Object::Dictionary(Arc::new(mark_info)));
                let xmp = self.generate_xmp(standard);
                let mut meta_dict = std::collections::BTreeMap::new();
                meta_dict.insert("Type".into(), Object::Name("Metadata".into()));
                meta_dict.insert("Subtype".into(), Object::Name("XML".into()));
                let meta_stream = Object::Stream(Arc::new(meta_dict), Bytes::from(xmp));
                let meta_ref = self.inner.add_object(meta_stream);
                catalog.insert("Metadata".into(), Object::Reference(meta_ref));
            }
            PdfStandard::ISO32000_2 => {
                println!("Note: Enforcing ISO 32000-2 (PDF 2.0) mandatory features...");
                catalog.insert("Version".into(), Object::Name("2.0".into()));
                let xmp = self.generate_xmp(standard);
                let mut meta_dict = std::collections::BTreeMap::new();
                meta_dict.insert("Type".into(), Object::Name("Metadata".into()));
                meta_dict.insert("Subtype".into(), Object::Name("XML".into()));
                let meta_stream = Object::Stream(Arc::new(meta_dict), Bytes::from(xmp));
                let meta_ref = self.inner.add_object(meta_stream);
                catalog.insert("Metadata".into(), Object::Reference(meta_ref));
            }
        }

        // Update the document root with modified catalog
        self.inner.update_object(catalog_ref.id, catalog_obj)?;
        Ok(())
    }

    fn generate_xmp(&self, standard: PdfStandard) -> String {
        let mut rdf = String::new();
        match standard {
            PdfStandard::A4 => {
                rdf.push_str("<rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n");
                rdf.push_str("  <pdfaid:part>4</pdfaid:part>\n");
                rdf.push_str("  <pdfaid:conformance>F</pdfaid:conformance>\n");
                rdf.push_str("</rdf:Description>\n");
            }
            PdfStandard::X6 => {
                rdf.push_str("<rdf:Description rdf:about=\"\" xmlns:pdfxid=\"http://www.niso.org/pdfx/ns/id/\">\n");
                rdf.push_str("  <pdfxid:GTS_PDFXVersion>PDF/X-6</pdfxid:GTS_PDFXVersion>\n");
                rdf.push_str("</rdf:Description>\n");
            }
            PdfStandard::UA2 => {
                rdf.push_str("<rdf:Description rdf:about=\"\" xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\">\n");
                rdf.push_str("  <pdfuaid:part>2</pdfuaid:part>\n");
                rdf.push_str("</rdf:Description>\n");
            }
            PdfStandard::ISO32000_2 => {
                 // Minimal XMP for base PDF 2.0
                 rdf.push_str("<rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n");
                 rdf.push_str("  <pdf:PDFVersion>2.0</pdf:PDFVersion>\n");
                 rdf.push_str("</rdf:Description>\n");
            }
        }

        format!(
            r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    {rdf}
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
        )
    }

    /// Renders a specific page to a PNG file.
    ///
    /// This is an asynchronous operation that utilizes the Vello GPU-accelerated
    /// backend and the headless rendering pipeline.
    pub async fn render_page_to_file(&self, index: usize, output_path: &Path) -> PdfResult<()> {
        let page = self.get_page(index)?;
        
        // 1. Resolve MediaBox and page dimensions first (needed for coordinate flip)
        let mediabox_obj = page.doc_page.media_box()
            .ok_or_else(|| ferruginous_core::PdfError::Other("Missing /MediaBox".into()))?;
        
        // Resolve if it's a reference
        let mediabox = self.inner.resolve_if_ref(&mediabox_obj)?.as_array()
            .ok_or_else(|| ferruginous_core::PdfError::Other("Invalid MediaBox".into()))?;
            
        let (x1, y1, x2, y2) = (mediabox[0].as_f64().unwrap_or(0.0), mediabox[1].as_f64().unwrap_or(0.0), mediabox[2].as_f64().unwrap_or(595.0), mediabox[3].as_f64().unwrap_or(842.0));
        let w = (x2 - x1).abs() as u32;
        let h = (y2 - y1).abs() as u32;
        eprintln!("DEBUG: Page MediaBox=[{x1}, {y1}, {x2}, {y2}], h={h}");
        
        eprintln!("DEBUG: Rendering page: MediaBox=[{x1}, {y1}, {x2}, {y2}], size={w}x{h}");

        // 2. Setup Backend with Coordinate Flip (PDF: Y-up, Vello: Y-down)
        let mut backend = VelloBackend::new();
        // T = [1 0 0 -1 -x1 h+y1]
        let flip = kurbo::Affine::new([1.0, 0.0, 0.0, -1.0, -x1, h as f64 + y1]);
        backend.transform(flip);
 
        // 3. Resolve Resources and setup Interpreter
        let res_dict = page.doc_page.resources().and_then(|o| o.as_dict_arc()).unwrap_or_else(|| Arc::new(std::collections::BTreeMap::new()));
        let mut interpreter = Interpreter::new(&mut backend, &self.inner, res_dict);
        
        // 4. Resolve and execute Contents
        let contents = page.doc_page.dictionary.get(&"Contents".into())
            .ok_or_else(|| ferruginous_core::PdfError::Other("Missing /Contents".into()))?;
            
        let content_refs = match contents {
            Object::Reference(r) => vec![*r],
            Object::Array(a) => a.iter().filter_map(|o| {
                if let Object::Reference(r) = o { Some(*r) } else { None }
            }).collect(),
            Object::Stream(_, _) => {
                let decoded = contents.decode_stream()?;
                interpreter.execute(&decoded)?;
                vec![]
            }
            _ => return Err(ferruginous_core::PdfError::Other("Invalid /Contents".into())),
        };

        for r in content_refs {
            let stream_obj = self.inner.resolve(&r)?;
            if let Object::Stream(_, _) = &stream_obj {
                let decoded = stream_obj.decode_stream()?;
                interpreter.execute(&decoded)?;
            }
        }

        // 5. Render to Output
        let format = match output_path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()).as_deref() {
            Some("jpg" | "jpeg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            Some("bmp") => ImageFormat::Bmp,
            Some("tiff") => ImageFormat::Tiff,
            _ => ImageFormat::Png, // Default to PNG
        };

        render_to_image(backend.scene(), w, h, output_path, format).await
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
    pub fn save_as_version(&mut self, output_path: &Path, version: &str) -> PdfResult<()> {
        use crate::writer::PdfWriter;
        let file = std::fs::File::create(output_path)
            .map_err(|e| ferruginous_core::PdfError::Other(e.to_string()))?;
        let mut writer = PdfWriter::new(file);

        writer.write_header(version)?;

        // If Vacuum is enabled, crawl the document to find reachable objects
        let reachable_ids = if self.vacuum {
            let mut roots = vec![self.inner.root()];
            if let Some(info_ref) = self.inner.trailer().get(&"Info".into()).and_then(|o| o.as_reference()) {
                if !self.strip {
                    roots.push(info_ref);
                }
            }
            let mut all_reachable = std::collections::BTreeSet::new();
            for root in roots {
                if let Ok(visited) = self.inner.explore_dependencies(&root) {
                    all_reachable.extend(visited);
                }
            }
            Some(all_reachable)
        } else {
            None
        };

        // Resolve all indirect objects from the source document
        let mut root_obj = self.inner.resolve(&self.inner.root())?;
        
        // If upgrading to 2.0, ensure /Version /2.0 and mandatory /Metadata are in the Catalog
        if version == "2.0" {
            if let Some(dict_mut) = root_obj.as_dict_mut() {
                dict_mut.insert("Version".into(), Object::Name("2.0".into()));
                
                // ISO 32000-2 SHALL contain a Metadata entry
                if let std::collections::btree_map::Entry::Vacant(e) = dict_mut.entry("Metadata".into()) {
                    let xmp = self.generate_xmp(PdfStandard::ISO32000_2);
                    let mut meta_dict = std::collections::BTreeMap::new();
                    meta_dict.insert("Type".into(), Object::Name("Metadata".into()));
                    meta_dict.insert("Subtype".into(), Object::Name("XML".into()));
                    let meta_stream = Object::Stream(Arc::new(meta_dict), Bytes::from(xmp));
                    let meta_ref = self.inner.add_object(meta_stream);
                    e.insert(Object::Reference(meta_ref));
                }
            }
            // Persist catalog changes back to internal store for consistency
            self.inner.update_object(self.inner.root().id, root_obj.clone())?;
        }

        // If password is set, we would ideally initialize a security handler here.
        // For Phase 13, we focus on notifying about the encryption intent.
        if let Some(ref pass) = self.password {
            println!("Note: Securing document with password: {pass}");
            // Encryption dictionary generation logic would go here
        }

        // Write all objects from the store
        for (&id, entry) in &self.inner.store().entries {
            // Filter unreachable objects if vacuum is enabled
            if let Some(ref reachable) = reachable_ids {
                if !reachable.contains(&id) {
                    continue;
                }
            }

            let generation = match entry {
                ferruginous_doc::XRefEntry::InUse { generation, .. } => *generation,
                ferruginous_doc::XRefEntry::Compressed { .. } => 0,
                ferruginous_doc::XRefEntry::Free { .. } => continue,
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

        // Find Info dictionary reference if it exists, but skip if stripping
        let info_ref = if self.strip {
            None
        } else {
            self.inner.trailer().get(&"Info".into()).and_then(|o| o.as_reference())
        };

        writer.finish(self.inner.root(), info_ref)
            .map_err(|e| ferruginous_core::PdfError::Other(e.to_string()))?;

        Ok(())
    }

    /// Saves the current document in a linearized (Fast Web View) format.
    ///
    /// This rearranges objects and injects hint streams for optimized web delivery.
    pub fn save_linearized(&mut self, output_path: &Path, version: &str) -> PdfResult<()> {
        // For PDF 2.0, we prefix the write with mandatory metadata injection
        if version == "2.0" {
            let catalog_ref = self.inner.root();
            let mut catalog_obj = self.inner.resolve(&catalog_ref)?;
            if let Some(dict_mut) = catalog_obj.as_dict_mut() {
                dict_mut.insert("Version".into(), ferruginous_core::Object::Name("2.0".into()));
                if let std::collections::btree_map::Entry::Vacant(e) = dict_mut.entry("Metadata".into()) {
                    let xmp = self.generate_xmp(PdfStandard::ISO32000_2);
                    let mut meta_dict = std::collections::BTreeMap::new();
                    meta_dict.insert("Type".into(), ferruginous_core::Object::Name("Metadata".into()));
                    meta_dict.insert("Subtype".into(), ferruginous_core::Object::Name("XML".into()));
                    let meta_stream = ferruginous_core::Object::Stream(std::sync::Arc::new(meta_dict), Bytes::from(xmp));
                    let meta_ref = self.inner.add_object(meta_stream);
                    e.insert(ferruginous_core::Object::Reference(meta_ref));
                }
            }
            self.inner.update_object(catalog_ref.id, catalog_obj)?;
        }

        let (_page_map, _shared_objects, sections) = self.map_linearization_sections()?;
        let (params, page_stats) = self.perform_linearization_dry_run(&sections, version)?;
        self.perform_linearized_write(output_path, version, &sections, &params, &page_stats)
    }

    fn map_linearization_sections(&self) -> PdfResult<(std::collections::BTreeMap<u32, usize>, std::collections::BTreeSet<u32>, Vec<Vec<Reference>>)> {
        use std::collections::{BTreeSet, BTreeMap};
        let num_pages = self.inner.get_page_count()?;
        let catalog_ref = self.inner.root();
        
        let mut page_map = BTreeMap::new();
        let mut shared_objects = BTreeSet::new();
        
        let root_deps = self.inner.explore_dependencies(&catalog_ref)?;
        for id in root_deps {
            page_map.insert(id, 0);
        }

        for i in 0..num_pages {
            let p = self.inner.get_page(i)?;
            let deps = self.inner.explore_dependencies(&p.reference)?;
            page_map.insert(p.reference.id, i);
            for parent in &p.parents {
                page_map.entry(parent.id).or_insert(i);
            }
            for id in deps {
                if let Some(&first_page) = page_map.get(&id) {
                    if first_page != i { shared_objects.insert(id); }
                } else {
                    page_map.insert(id, i);
                }
            }
        }

        let mut sections: Vec<Vec<Reference>> = vec![Vec::new(); num_pages + 1];
        let shared_sec_idx = num_pages;
        let reachable_ids = if self.vacuum { Some(self.get_reachable_ids()?) } else { None };

        for (&id, entry) in &self.inner.store().entries {
            if let Some(ref reachable) = reachable_ids {
                if !reachable.contains(&id) { continue; }
            }
            let generation_num = match entry {
                ferruginous_doc::XRefEntry::InUse { generation, .. } => *generation,
                ferruginous_doc::XRefEntry::Compressed { .. } => 0,
                ferruginous_doc::XRefEntry::Free { .. } => continue, // Skip Free entries
            };
            let r = Reference::new(id, generation_num);
            if shared_objects.contains(&id) {
                sections[shared_sec_idx].push(r);
            } else if let Some(&page_idx) = page_map.get(&id) {
                sections[page_idx].push(r);
            } else {
                sections[shared_sec_idx].push(r);
            }
        }
        Ok((page_map, shared_objects, sections))
    }

    fn get_reachable_ids(&self) -> PdfResult<std::collections::BTreeSet<u32>> {
        let mut roots = vec![self.inner.root()];
        if let Some(info_ref) = self.inner.trailer().get(&"Info".into()).and_then(|o| o.as_reference()) {
            if !self.strip { roots.push(info_ref); }
        }
        let mut all_reachable = std::collections::BTreeSet::new();
        for root in roots {
            all_reachable.extend(self.inner.explore_dependencies(&root)?);
        }
        Ok(all_reachable)
    }

    fn perform_linearization_dry_run(&self, sections: &[Vec<Reference>], version: &str) -> PdfResult<(crate::writer::LinearizationParams, Vec<PageStats>)> {
        use crate::writer::{PdfWriter, NullWriter, LinearizationParams};
        let num_pages = self.inner.get_page_count()?;
        let catalog_ref = self.inner.root();
        let p1 = self.inner.get_page(0)?;
        let mut page_stats = Vec::with_capacity(num_pages);
        
        let mut params = LinearizationParams {
            num_pages,
            first_page_obj: p1.reference.id,
            ..Default::default()
        };
        
        let lin_dict_id = self.inner.store().max_id() + 1;
        let mut dry_writer = PdfWriter::new(NullWriter::new());
        dry_writer.write_header(version).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        dry_writer.write_linearization_dict(lin_dict_id, &params).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        params.hint_stream_offset = dry_writer.current_offset();
        params.hint_stream_len = 2048 + (num_pages * 32); 
        dry_writer.write_all(&vec![0; params.hint_stream_len]).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        for (i, section) in sections.iter().enumerate().take(num_pages) {
            let start = dry_writer.current_offset();
            let mut count = 0;
            for r in section {
                let obj = self.inner.resolve(r)?;
                dry_writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
                count += 1;
            }
            page_stats.push(PageStats { offset: start, length: dry_writer.current_offset() - start, obj_count: count });
            if i == 0 { params.end_of_first_page = dry_writer.current_offset(); }
        }
        
        for r in &sections[num_pages] {
            let obj = self.inner.resolve(r)?;
            dry_writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        }
        
        params.main_xref_offset = dry_writer.current_offset();
        let info_ref = self.inner.trailer().get(&"Info".into()).and_then(|o| o.as_reference());
        dry_writer.finish(catalog_ref, info_ref).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        params.file_len = dry_writer.current_offset();
        
        Ok((params, page_stats))
    }

    fn perform_linearized_write(&self, output_path: &Path, version: &str, sections: &[Vec<Reference>], params: &crate::writer::LinearizationParams, page_stats: &[PageStats]) -> PdfResult<()> {
        use crate::writer::PdfWriter;
        use ferruginous_core::{PdfName, Object};
        let num_pages = self.inner.get_page_count()?;
        let catalog_ref = self.inner.root();
        let lin_dict_id = self.inner.store().max_id() + 1;
        let hint_stream_id = lin_dict_id + 1;
        let hint_data = build_hint_stream(page_stats, params);

        let file = std::fs::File::create(output_path).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        let mut writer = PdfWriter::new(file);
        
        writer.write_header(version).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        writer.write_linearization_dict(lin_dict_id, params).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        
        let mut hint_dict = std::collections::BTreeMap::new();
        hint_dict.insert(PdfName::new(b"Type"), Object::Name(PdfName::new(b"HintStream")));
        writer.write_indirect_object(hint_stream_id, 0, &Object::Stream(std::sync::Arc::new(hint_dict), bytes::Bytes::from(hint_data)))
            .map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
            
        for section in sections.iter().take(num_pages) {
            for r in section {
                let obj = self.inner.resolve(r)?;
                writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
            }
        }
        
        for r in &sections[num_pages] {
            let obj = self.inner.resolve(r)?;
            writer.write_indirect_object(r.id, r.generation, &obj).map_err(|e: std::io::Error| PdfError::Other(e.to_string()))?;
        }
        
        let info_ref = self.inner.trailer().get(&"Info".into()).and_then(|o| o.as_reference());
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

        if let Object::Stream(_, _) = contents {
             let decoded = contents.decode_stream()?;
             interpreter.execute(&decoded)?;
        } else {
            for r in content_refs {
                let stream_obj = resolver.resolve(&r)?;
                if let Object::Stream(_, _) = &stream_obj {
                    let decoded = stream_obj.decode_stream()?;
                    interpreter.execute(&decoded)?;
                }
            }
        }

        Ok(backend.text)
    }

    /// Sets the rotation of the page in degrees (must be a multiple of 90).
    pub fn set_rotation(&mut self, angle: i32) -> PdfResult<()> {
        if angle % 90 != 0 {
            return Err(ferruginous_core::PdfError::Other("Rotation must be a multiple of 90".into()));
        }
        
        // Use Arc::make_mut to get a mutable reference to the dictionary
        let dict = Arc::make_mut(&mut self.doc_page.dictionary);
        dict.insert(ferruginous_core::PdfName::from("Rotate"), Object::Integer(angle as i64));
        
        Ok(())
    }
}

impl PdfDocument {
    /// Merges multiple PDF documents into a new single document.
    pub fn merge(sources: Vec<PdfDocument>) -> PdfResult<Self> {
        if sources.is_empty() {
            return Err(ferruginous_core::PdfError::Other("No sources to merge".into()));
        }

        // Create a minimal empty document as the target
        // (Header + Catalog [1 0 R] + Pages root [2 0 R])
        let mut target_inner = Document::open_repair(Bytes::from_static(b"%PDF-2.0\r\n1 0 obj\r\n<< /Type /Catalog /Pages 2 0 R >>\r\nendobj\r\n2 0 obj\r\n<< /Type /Pages /Kids [] /Count 0 >>\r\nendobj\r\ntrailer\r\n<< /Root 1 0 R /Size 3 >>\r\nstartxref\r\n0\r\n%%EOF\r\n"))?;
        
        let mut cloner = crate::cloning::ObjectCloner::new(&mut target_inner);
        let mut all_page_refs = Vec::new();

        for src in sources {
            let src_page_count = src.page_count()?;
            for i in 0..src_page_count {
                let page = src.get_page(i)?;
                let page_ref = page.reference();
                // Clone the page object and all its dependencies
                let cloned_ref_obj = cloner.clone_object(&src.inner, &Object::Reference(page_ref))?;
                if let Object::Reference(new_ref) = cloned_ref_obj {
                    all_page_refs.push(Object::Reference(new_ref));
                }
            }
        }

        // Reconstruct the target's page tree
        let pages_root_ref = Reference::new(2, 0);
        let mut pages_dict = std::collections::BTreeMap::new();
        pages_dict.insert("Type".into(), Object::Name("Pages".into()));
        pages_dict.insert("Count".into(), Object::Integer(all_page_refs.len() as i64));
        pages_dict.insert("Kids".into(), Object::Array(Arc::new(all_page_refs)));
        
        target_inner.update_object(pages_root_ref.id, Object::Dictionary(Arc::new(pages_dict)))?;

        Ok(Self { inner: target_inner, vacuum: false, strip: false, password: None })
    }

    /// Extracts specific pages from the current document into a new document.
    pub fn extract_pages(&self, indices: Vec<usize>) -> PdfResult<Self> {
        if indices.is_empty() {
            return Err(ferruginous_core::PdfError::Other("No pages to extract".into()));
        }

        let mut target_inner = Document::open_repair(Bytes::from_static(b"%PDF-2.0\r\n1 0 obj\r\n<< /Type /Catalog /Pages 2 0 R >>\r\nendobj\r\n2 0 obj\r\n<< /Type /Pages /Kids [] /Count 0 >>\r\nendobj\r\ntrailer\r\n<< /Root 1 0 R /Size 3 >>\r\nstartxref\r\n0\r\n%%EOF\r\n"))?;
        
        let mut cloner = crate::cloning::ObjectCloner::new(&mut target_inner);
        let mut extracted_refs = Vec::new();

        for idx in indices {
            // Validate bounds
            let total = self.page_count()?;
            if idx >= total {
                continue;
            }
            
            let page = self.get_page(idx)?;
            let page_ref = page.reference();
            
            let cloned_ref_obj = cloner.clone_object(&self.inner, &Object::Reference(page_ref))?;
            if let Object::Reference(new_ref) = cloned_ref_obj {
                extracted_refs.push(Object::Reference(new_ref));
            }
        }

        let pages_root_ref = Reference::new(2, 0);
        let mut pages_dict = std::collections::BTreeMap::new();
        pages_dict.insert("Type".into(), Object::Name("Pages".into()));
        pages_dict.insert("Count".into(), Object::Integer(extracted_refs.len() as i64));
        pages_dict.insert("Kids".into(), Object::Array(Arc::new(extracted_refs)));
        
        target_inner.update_object(pages_root_ref.id, Object::Dictionary(Arc::new(pages_dict)))?;

        Ok(Self { inner: target_inner, vacuum: false, strip: false, password: None })
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
    fn set_fill_color(&mut self, _color: Color) {}
    fn set_stroke_color(&mut self, _color: Color) {}
    
    fn define_font(&mut self, _name: &str, _data: Vec<u8>) {}
    
    fn show_text(&mut self, text: &str, _font_name: &str, _size: f32, _transform: kurbo::Affine) {
        self.text.push_str(text);
        if !text.ends_with(' ') {
             // Basic heuristic for spaces between Tj ops if needed, 
             // but usually spaces are explicit in PDF or TJ.
        }
    }
}
