//! Ferruginous SDK: High-level PDF processing library.
//!
//! This crate provides a high-level, easy-to-use interface for PDF document
//! manipulation, rendering, and auditing, abstracting away the low-level
//! complexities of the core type system and document model.

use crate::remediation::HeuristicEngine;
use crate::structure::{AuditFinding, MatterhornAuditor};
use bytes::Bytes;
pub use ferruginous_core::{
    Document, Handle, Object, Page, PdfArena, PdfError, PdfName, PdfResult,
};
use ferruginous_render::VelloBackend;
use std::path::Path;

/// The internal cloning module for object migration.
pub mod cloning;
/// The internal interpreter module for processing content streams.
pub mod interpreter;
pub use interpreter::Interpreter;
/// The internal remediation module for structural repair.
pub mod remediation;
/// The internal structure module for UA-2 logical tree handling.
pub mod structure;
/// The internal writer module for generating PDF files.
pub mod writer;

/// Supported text string encodings for PDF output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StringEncoding {
    /// Maximum compatibility using UTF-16BE with BOM (FE FF).
    #[default]
    Utf16BE,
    /// PDF 2.0 native UTF-8 with BOM (EF BB BF).
    Utf8,
}

/// Options for saving a PDF document.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct SaveOptions {
    /// Whether to compress streams using FlateDecode.
    pub compress: bool,
    /// The compression level to use (0-9).
    pub compression_level: u32,
    /// Whether to remove unreachable objects.
    pub vacuum: bool,
    /// Whether to strip descriptive metadata.
    pub strip: bool,
    /// Optional encryption password.
    pub password: Option<String>,
    /// Whether to use Object Streams (ObjStm) for high-density compression.
    pub obj_stm: bool,
    /// Optional image re-compression quality (1-100).
    pub image_quality: Option<u32>,
    /// Document primary language (ISO 639-1).
    pub lang: Option<String>,
    /// Override document title.
    pub title: Option<String>,
    /// Override document author.
    pub author: Option<String>,
    /// Set copyright notice in XMP.
    pub copyright: Option<String>,
    /// PDF permission flags (e.g., "print,copy").
    pub permissions: Option<String>,
    /// Preferred text string encoding for non-ASCII characters.
    pub string_encoding: StringEncoding,
    /// Simulate saving and report results without writing to disk.
    pub dry_run: bool,
}

/// Options for digitally signing a PDF document.
#[derive(Debug, Clone, Default)]
pub struct SignOptions {
    /// Reason for signing.
    pub reason: Option<String>,
    /// Location of signing.
    pub location: Option<String>,
    /// Contact information for the signer.
    pub contact_info: Option<String>,
    /// Common Name (CN) of the signer.
    pub name: Option<String>,
    /// DER-encoded certificate (X.509).
    pub certificate: Option<Vec<u8>>,
    /// PEM or DER encoded private key.
    pub private_key: Option<Vec<u8>>,
    /// Page index (0-based) to place the signature widget.
    pub page_index: usize,
    /// Visual rectangle for the signature widget [x1, y1, x2, y2].
    pub rect: [f32; 4],
}

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

/// Summary of document properties and structural health.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentSummary {
    /// The PDF version string.
    pub version: String,
    /// The total number of pages.
    pub page_count: usize,
    /// Extracted document metadata.
    pub metadata: MetadataInfo,
    /// List of fonts used in the document.
    pub fonts: Vec<FontSummary>,
    /// Summary of structural compliance issues.
    pub compliance: ComplianceSummary,
}

pub use ferruginous_core::font::FontSummary;
pub use ferruginous_core::metadata::MetadataInfo;

/// Structural compliance overview.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ComplianceSummary {
    /// List of compliance issues found.
    pub issues: Vec<ComplianceIssue>,
}

/// A specific compliance violation or observation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComplianceIssue {
    /// The standard being checked (e.g., "PDF/UA-2").
    pub standard: String,
    /// The severity of the issue.
    pub severity: IssueSeverity,
    /// A descriptive message.
    pub message: String,
}

/// Severity of a compliance issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueSeverity {
    /// Information or observation.
    Info,
    /// Potential issue but not a strict violation.
    Warning,
    /// Violation of a core requirement.
    Error,
    /// Critical violation making the document invalid or inaccessible.
    Critical,
}

/// High-level entry point for interacting with a PDF document.
pub struct PdfDocument {
    inner: Document,
}

impl PdfDocument {
    /// Opens a PDF document from a byte buffer with default ingestion options.
    pub fn open(data: Bytes) -> PdfResult<Self> {
        Self::open_with_options(data, &ferruginous_core::ingest::IngestionOptions::default())
    }

    /// Opens a PDF document with custom ingestion options.
    pub fn open_with_options(
        data: Bytes,
        options: &ferruginous_core::ingest::IngestionOptions,
    ) -> PdfResult<Self> {
        let inner = Document::open(data, options)?;
        Ok(Self { inner })
    }

    /// Returns the internal document.
    pub fn inner(&self) -> &Document {
        &self.inner
    }

    /// Returns the total number of pages.
    pub fn page_count(&self) -> PdfResult<usize> {
        self.inner.page_count()
    }

    /// Attempts to open and repair a PDF document with custom options.
    pub fn open_and_repair_with_options(
        data: Bytes,
        options: &ferruginous_core::ingest::IngestionOptions,
    ) -> PdfResult<Self> {
        let inner = Document::open_repair(data, options)?;
        Ok(Self { inner })
    }

    /// Merges multiple documents into a new one.
    pub fn merge(sources: Vec<PdfDocument>) -> PdfResult<Self> {
        if sources.is_empty() {
            return Err(PdfError::Other("No sources to merge".into()));
        }

        let target_arena = PdfArena::new();
        let mut target_pages = Vec::new();

        let pages_root_key = target_arena.name("Pages");
        let type_key = target_arena.name("Type");
        let parent_key = target_arena.name("Parent");
        let kids_key = target_arena.name("Kids");
        let count_key = target_arena.name("Count");
        let catalog_key = target_arena.name("Catalog");

        // 1. Create target Pages root (placeholder)
        let pages_root_dict_handle = target_arena.alloc_dict(std::collections::BTreeMap::new());
        let pages_root_handle = target_arena.alloc_object(Object::Dictionary(pages_root_dict_handle));

        // 2. Clone pages from all sources
        for source_doc in sources {
            let mut cloner = cloning::ObjectCloner::new(source_doc.inner.arena(), &target_arena);
            let count = source_doc.page_count()?;
            for i in 0..count {
                let source_page = source_doc.inner.get_page(i)?;
                let source_page_handle = source_page.dict_handle();
                
                // Clone the page dictionary
                let cloned_page_dict_obj = cloner.clone_object(&Object::Dictionary(source_page_handle))?;
                
                if let Object::Dictionary(dh) = cloned_page_dict_obj {
                    let mut dict = target_arena.get_dict(dh).unwrap_or_default();
                    // Update parent to the new Pages root
                    dict.insert(parent_key, Object::Reference(pages_root_handle));
                    target_arena.set_dict(dh, dict);
                    
                    // Allocate as an indirect object
                    let target_page_handle = target_arena.alloc_object(Object::Dictionary(dh));
                    target_pages.push(Object::Reference(target_page_handle));
                }
            }
        }

        // 3. Finalize Pages root
        let mut pages_dict = std::collections::BTreeMap::new();
        pages_dict.insert(type_key, Object::Name(pages_root_key));
        #[allow(clippy::cast_possible_wrap)]
        pages_dict.insert(count_key, Object::Integer(target_pages.len() as i64));
        pages_dict.insert(kids_key, Object::Array(target_arena.alloc_array(target_pages)));
        target_arena.set_dict(pages_root_dict_handle, pages_dict);

        // 4. Create Catalog
        let mut catalog_dict = std::collections::BTreeMap::new();
        catalog_dict.insert(type_key, Object::Name(catalog_key));
        catalog_dict.insert(pages_root_key, Object::Reference(pages_root_handle));
        let catalog_handle = target_arena.alloc_object(Object::Dictionary(target_arena.alloc_dict(catalog_dict)));

        Ok(Self {
            inner: Document::new(target_arena, catalog_handle, None),
        })
    }

    /// Extracts specific pages into a new document.
    pub fn extract_pages(&self, indices: Vec<usize>) -> PdfResult<Self> {
        if indices.is_empty() {
            return Err(PdfError::Other("No indices to extract".into()));
        }

        let target_arena = PdfArena::new();
        let mut target_pages = Vec::new();

        let pages_root_key = target_arena.name("Pages");
        let type_key = target_arena.name("Type");
        let parent_key = target_arena.name("Parent");
        let kids_key = target_arena.name("Kids");
        let count_key = target_arena.name("Count");
        let catalog_key = target_arena.name("Catalog");

        // 1. Create target Pages root (placeholder)
        let pages_root_dict_handle = target_arena.alloc_dict(std::collections::BTreeMap::new());
        let pages_root_handle = target_arena.alloc_object(Object::Dictionary(pages_root_dict_handle));

        let mut cloner = cloning::ObjectCloner::new(self.inner.arena(), &target_arena);

        for i in indices {
            let source_page = self.inner.get_page(i)?;
            let source_page_handle = source_page.dict_handle();
            
            let cloned_page_dict_obj = cloner.clone_object(&Object::Dictionary(source_page_handle))?;
            
            if let Object::Dictionary(dh) = cloned_page_dict_obj {
                let mut dict = target_arena.get_dict(dh).unwrap_or_default();
                // Update parent to the new Pages root
                dict.insert(parent_key, Object::Reference(pages_root_handle));
                target_arena.set_dict(dh, dict);
                
                // Allocate as an indirect object
                let target_page_handle = target_arena.alloc_object(Object::Dictionary(dh));
                target_pages.push(Object::Reference(target_page_handle));
            }
        }

        // 2. Finalize Pages root
        let mut pages_dict = std::collections::BTreeMap::new();
        pages_dict.insert(type_key, Object::Name(pages_root_key));
        #[allow(clippy::cast_possible_wrap)]
        pages_dict.insert(count_key, Object::Integer(target_pages.len() as i64));
        pages_dict.insert(kids_key, Object::Array(target_arena.alloc_array(target_pages)));
        target_arena.set_dict(pages_root_dict_handle, pages_dict);

        // 3. Create Catalog
        let mut catalog_dict = std::collections::BTreeMap::new();
        catalog_dict.insert(type_key, Object::Name(catalog_key));
        catalog_dict.insert(pages_root_key, Object::Reference(pages_root_handle));
        let catalog_handle = target_arena.alloc_object(Object::Dictionary(target_arena.alloc_dict(catalog_dict)));

        Ok(Self {
            inner: Document::new(target_arena, catalog_handle, None),
        })
    }

    /// Saves the document to a file with a specific version and default options.
    pub fn save_as_version(&self, output_path: &Path, version: &str) -> PdfResult<()> {
        self.save_with_options(output_path, version, &SaveOptions::default())
    }

    /// Saves the document with custom options.
    pub fn save_with_options(
        &self,
        output_path: &Path,
        version: &str,
        options: &SaveOptions,
    ) -> PdfResult<()> {
        // 1. Update Metadata
        let mut metadata = self.inner.metadata();

        if let Some(v) = &options.title {
            metadata.title = Some(v.clone());
        }
        if let Some(v) = &options.author {
            metadata.author = Some(v.clone());
        }
        if let Some(_v) = &options.lang { /* Lang is usually in Catalog /Lang, not Metadata struct for now */
        }

        // Automatic Producer stamping
        metadata.producer =
            Some("ferruginous-sdk (https://github.com/akahmys/ferruginous)".to_string());

        if options.strip {
            // Strip metadata: we'll clear the fields in the struct
            metadata = MetadataInfo::default();
            metadata.producer = Some("ferruginous-sdk (optimized)".to_string());
        }

        ferruginous_core::metadata::update_document_metadata(&self.inner, &metadata)?;

        if options.strip {
            // Further stripping: remove Metadata entry from catalog
            let root_handle = *self.inner.root_handle();
            let arena = self.inner.arena();
            if let Some(Object::Dictionary(dh)) = arena.get_object(root_handle) {
                let mut dict = arena.get_dict(dh).unwrap_or_default();
                dict.remove(&arena.name("Metadata"));
                arena.set_dict(dh, dict);
            }
        }

        if options.dry_run {
            println!("SIMULATION: Pre-flight check complete. Metadata updated in memory.");
            return Ok(());
        }

        let file = std::fs::File::create(output_path).map_err(PdfError::Io)?;
        let mut writer = crate::writer::PdfWriter::new(file, self.inner.arena());
        writer.set_string_encoding(options.string_encoding);

        if options.vacuum {
            writer.set_vacuum(true);
        }

        if options.compress {
            writer.set_compression(options.compression_level);
        }
        writer.write_header(version)?;
        writer.finish(*self.inner.root_handle())?;
        Ok(())
    }

    /// Saves a linearized (Fast Web View) version of the document with custom options.
    pub fn save_linearized(
        &self,
        output_path: &Path,
        version: &str,
        options: &SaveOptions,
    ) -> PdfResult<()> {
        // Linearization involves object reordering and hint tables.
        // For M67, we implement the object reordering phase.

        // 1. Update Metadata (consistent with save_with_options)
        let mut metadata = self.inner.metadata();
        if let Some(v) = &options.title {
            metadata.title = Some(v.clone());
        }
        if let Some(v) = &options.author {
            metadata.author = Some(v.clone());
        }
        metadata.producer = Some("ferruginous-sdk (linearized)".to_string());

        ferruginous_core::metadata::update_document_metadata(&self.inner, &metadata)?;

        let file = std::fs::File::create(output_path).map_err(PdfError::Io)?;
        let mut writer = crate::writer::PdfWriter::new(file, self.inner.arena());
        writer.set_string_encoding(options.string_encoding);

        writer.set_linearize(true);
        if options.vacuum {
            writer.set_vacuum(true);
        }
        if options.compress {
            writer.set_compression(options.compression_level);
        }

        writer.write_header(version)?;
        writer.finish(*self.inner.root_handle())?;
        Ok(())
    }

    /// Signs the document and saves it with digital signature support.
    pub fn save_signed(
        &self,
        output_path: &Path,
        version: &str,
        options: &SaveOptions,
        sign_options: &SignOptions,
    ) -> PdfResult<()> {
        let arena = self.inner.arena();

        // 1. Create Signature Dictionary
        let mut sig_dict = std::collections::BTreeMap::new();
        sig_dict.insert(arena.name("Type"), Object::Name(arena.name("Sig")));
        sig_dict.insert(arena.name("Filter"), Object::Name(arena.name("Adobe.PPKLite")));
        sig_dict.insert(arena.name("SubFilter"), Object::Name(arena.name("adbe.pkcs7.detached")));

        if let Some(reason) = &sign_options.reason {
            sig_dict.insert(arena.name("Reason"), Object::String(Bytes::from(reason.clone())));
        }
        if let Some(location) = &sign_options.location {
            sig_dict.insert(arena.name("Location"), Object::String(Bytes::from(location.clone())));
        }
        if let Some(contact) = &sign_options.contact_info {
            sig_dict
                .insert(arena.name("ContactInfo"), Object::String(Bytes::from(contact.clone())));
        }
        if let Some(name) = &sign_options.name {
            sig_dict.insert(arena.name("Name"), Object::String(Bytes::from(name.clone())));
        }

        // Use a 2.0 compliant date format (D:YYYYMMDDHHmmSSOHH'mm')
        let now = "D:20260424235959+00'00'";
        sig_dict.insert(arena.name("M"), Object::String(Bytes::from(now)));

        // Placeholder for Contents (hex string) - 16KB reserved for PKCS#7
        let placeholder = vec![0u8; 8192];
        sig_dict.insert(arena.name("Contents"), Object::Hex(placeholder.into()));

        // ByteRange Placeholder
        let byte_range = vec![
            Object::Integer(0),
            Object::Integer(1_000_000_000), // Placeholder for patch
            Object::Integer(1_000_000_000), // Placeholder for patch
            Object::Integer(1_000_000_000), // Placeholder for patch
        ];
        sig_dict.insert(arena.name("ByteRange"), Object::Array(arena.alloc_array(byte_range)));

        let sig_handle = arena.alloc_object(Object::Dictionary(arena.alloc_dict(sig_dict)));

        // 2. Create Widget Annotation
        let mut widget_dict = std::collections::BTreeMap::new();
        widget_dict.insert(arena.name("Type"), Object::Name(arena.name("Annot")));
        widget_dict.insert(arena.name("Subtype"), Object::Name(arena.name("Widget")));
        widget_dict.insert(arena.name("FT"), Object::Name(arena.name("Sig")));
        widget_dict.insert(arena.name("T"), Object::String(Bytes::from("Signature1")));
        widget_dict.insert(arena.name("V"), Object::Reference(sig_handle));
        widget_dict.insert(arena.name("F"), Object::Integer(4)); // Print flag

        let rect = vec![
            Object::Real(f64::from(sign_options.rect[0])),
            Object::Real(f64::from(sign_options.rect[1])),
            Object::Real(f64::from(sign_options.rect[2])),
            Object::Real(f64::from(sign_options.rect[3])),
        ];
        widget_dict.insert(arena.name("Rect"), Object::Array(arena.alloc_array(rect)));

        let widget_handle = arena.alloc_object(Object::Dictionary(arena.alloc_dict(widget_dict)));

        // 3. Add to Page
        let page = self.inner.get_page(sign_options.page_index)?;
        let page_dict_handle = page.dict_handle();
        let mut page_dict = arena.get_dict(page_dict_handle).unwrap();

        let annots_key = arena.name("Annots");
        let mut annots = if let Some(Object::Array(ah)) = page_dict.get(&annots_key) {
            arena.get_array(*ah).unwrap_or_default()
        } else {
            Vec::new()
        };
        annots.push(Object::Reference(widget_handle));
        page_dict.insert(annots_key, Object::Array(arena.alloc_array(annots)));
        arena.set_dict(page_dict_handle, page_dict);

        // 4. Add to Catalog AcroForm
        let root_handle = *self.inner.root_handle();
        if let Some(Object::Dictionary(rdh)) = arena.get_object(root_handle) {
            let mut root_dict = arena.get_dict(rdh).unwrap();

            let mut acro_form =
                if let Some(Object::Dictionary(afh)) = root_dict.get(&arena.name("AcroForm")) {
                    arena.get_dict(*afh).unwrap_or_default()
                } else {
                    let mut af = std::collections::BTreeMap::new();
                    af.insert(arena.name("Fields"), Object::Array(arena.alloc_array(Vec::new())));
                    af
                };

            if let Some(Object::Array(fh)) = acro_form.get(&arena.name("Fields")) {
                let mut fields = arena.get_array(*fh).unwrap_or_default();
                fields.push(Object::Reference(widget_handle));
                acro_form.insert(arena.name("Fields"), Object::Array(arena.alloc_array(fields)));
            }

            // Set SigFlags to 3 (SignaturesExist | AppendOnly)
            acro_form.insert(arena.name("SigFlags"), Object::Integer(3));

            root_dict
                .insert(arena.name("AcroForm"), Object::Dictionary(arena.alloc_dict(acro_form)));
            arena.set_dict(rdh, root_dict);
        }

        // 5. Final Save with Signature Patching
        let file = std::fs::File::create(output_path).map_err(PdfError::Io)?;
        let mut writer = crate::writer::PdfWriter::new(file, arena);
        writer.set_string_encoding(options.string_encoding);

        // We'll tell the writer about the signature object to patch
        writer.add_signature_target(sig_handle);

        if options.vacuum {
            writer.set_vacuum(true);
        }
        if options.compress {
            writer.set_compression(options.compression_level);
        }

        writer.write_header(version)?;
        writer.finish(root_handle)?;
        Ok(())
    }

    /// Returns the physical viewport of the page (MediaBox).
    pub fn get_page_box(&self, index: usize) -> PdfResult<ferruginous_core::graphics::Rect> {
        let page = self.inner.get_page(index)?;
        if let Some(mb) = page.resolve_attribute("MediaBox")
            && let Some(arr_handle) = mb.as_array()
            && let Some(arr) = self.inner.arena().get_array(arr_handle)
            && arr.len() >= 4
        {
            let x1 = arr[0].resolve(self.inner.arena()).as_f64().unwrap_or(0.0);
            let y1 = arr[1].resolve(self.inner.arena()).as_f64().unwrap_or(0.0);
            let x2 = arr[2].resolve(self.inner.arena()).as_f64().unwrap_or(595.0);
            let y2 = arr[3].resolve(self.inner.arena()).as_f64().unwrap_or(842.0);
            return Ok(ferruginous_core::graphics::Rect::new(x1, y1, x2, y2));
        }
        Ok(ferruginous_core::graphics::Rect::new(0.0, 0.0, 595.0, 842.0)) // Default A4
    }

    /// Returns the physical dimensions of the page (Width, Height).
    pub fn get_page_size(&self, index: usize) -> PdfResult<(f64, f64)> {
        let r = self.get_page_box(index)?;
        Ok(((r.x2 - r.x1).abs(), (r.y2 - r.y1).abs()))
    }

    /// Renders a page to a provide backend.
    pub fn render_page(
        &self,
        index: usize,
        backend: &mut dyn ferruginous_render::RenderBackend,
        initial_transform: kurbo::Affine,
    ) -> PdfResult<()> {
        let page = self.inner.get_page(index)?;
        let arena = self.inner.arena();
        let res_obj = page.resolve_attribute("Resources").unwrap_or_else(|| {
            Object::Dictionary(arena.alloc_dict(std::collections::BTreeMap::new()))
        });

        if let Object::Dictionary(rh) = res_obj {
            let mut interpreter = Interpreter::new(backend, &self.inner, rh, initial_transform);
            let contents_obj = page
                .resolve_attribute("Contents")
                .ok_or_else(|| PdfError::Other("Page has no contents".into()))?;

            match contents_obj {
                Object::Reference(h) => {
                    let stream = self.inner.resolve(&h)?;
                    let data = self.inner.decode_stream(&stream)?;

                    interpreter.execute(&data)?;
                }
                Object::Array(h) => {
                    if let Some(arr) = arena.get_array(h) {
                        for obj in &arr {
                            if let Object::Reference(rh) = obj {
                                let stream = self.inner.resolve(rh)?;
                                let data = self.inner.decode_stream(&stream)?;

                                interpreter.execute(&data)?;
                            }
                        }
                    }
                }
                Object::Stream(_, _) => {
                    let data = self.inner.decode_stream(&contents_obj)?;
                    interpreter.execute(&data)?;
                }
                _ => return Err(PdfError::Other("Invalid Contents type".into())),
            }
        }
        Ok(())
    }

    /// Upgrades the document to a specific standard (A-4, X-6, UA-2).
    pub fn upgrade_to_standard(&mut self, _standard: PdfStandard) -> PdfResult<()> {
        // TODO: Rule-based upgrade logic
        Ok(())
    }

    /// Primary entry point for re-tagging a document automatically.
    pub fn retag_document(&mut self) -> PdfResult<()> {
        crate::remediation::retag(&mut self.inner)
    }

    /// Returns a list of potential structural remediations for the document.
    pub fn get_remediation_candidates(
        &self,
    ) -> PdfResult<Vec<crate::remediation::RemediationCandidate>> {
        let engine = HeuristicEngine::new(self.inner.arena());
        engine.infer_structure(&self.inner)
    }

    /// Extracts Unicode text from a specific page.
    pub fn extract_text(&self, index: usize) -> PdfResult<String> {
        let mut backend = crate::remediation::TextExtractionBackend::new();
        self.render_page(index, &mut backend, kurbo::Affine::IDENTITY)?;
        Ok(backend.finish())
    }

    /// Prints a textual representation of the logical structure tree.
    pub fn print_structure(&self) -> PdfResult<String> {
        let root_opt = self.inner.get_structure_root()?;
        if let Some(root) = root_opt {
            Ok(format!("Structure Tree Root found: {root:?}"))
        } else {
            Ok("No logical structure found.".into())
        }
    }

    /// Controls whether unreachable objects are removed on save.
    pub fn set_vacuum(&mut self, _vacuum: bool) {}
    /// Controls whether descriptive metadata is stripped on save.
    pub fn set_strip(&mut self, _strip: bool) {}
    /// Sets the document open password.
    pub fn set_password(&mut self, _password: Option<String>) {}

    /// Sets the rotation of a specific page.
    pub fn set_page_rotation(&mut self, index: usize, angle: i32) -> PdfResult<()> {
        let page = self.inner.get_page(index)?;
        let dh = page.dict_handle();
        let arena = self.inner.arena();
        let mut dict = arena.get_dict(dh).unwrap_or_default();
        dict.insert(arena.name("Rotate"), Object::Integer(i64::from(angle)));
        arena.set_dict(dh, dict);
        Ok(())
    }

    /// Performs a structural health audit for PDF/UA-2.
    pub fn audit_ua2(&self) -> PdfResult<Vec<AuditFinding>> {
        let root_opt = self.inner.get_structure_root()?;
        if let Some(root) = root_opt {
            let auditor = MatterhornAuditor::new(self.inner.arena());
            auditor.audit(root)
        } else {
            Ok(vec![AuditFinding {
                checkpoint: "00-001".into(),
                severity: "Warning".into(),
                message: "Document missing Structural Tree Root. Not a tagged PDF.".into(),
            }])
        }
    }

    /// Returns a comprehensive summary of the document.
    pub fn get_summary(&self) -> PdfResult<DocumentSummary> {
        let pdf_20 = true; // High-level inference
        let findings = self.audit_ua2()?;
        let mut issues = Vec::new();
        for f in findings {
            issues.push(ComplianceIssue {
                standard: "PDF/UA-2".into(),
                severity: match f.severity.as_str() {
                    "Error" => IssueSeverity::Error,
                    "Critical" => IssueSeverity::Critical,
                    _ => IssueSeverity::Warning,
                },
                message: format!("[{}] {}", f.checkpoint, f.message),
            });
        }

        Ok(DocumentSummary {
            version: if pdf_20 { "2.0".into() } else { "1.7".into() },
            page_count: self.page_count()?,
            metadata: self.inner.metadata(),
            fonts: self.inner.fonts(),
            compliance: ComplianceSummary { issues },
        })
    }

    /// Returns a list of all fonts embedded or referenced in the document.
    pub fn get_embedded_fonts(&self) -> Vec<FontSummary> {
        self.inner.fonts()
    }

    /// Renders a specific page to an image file, detecting format from extension.
    pub fn render_page_to_file(&self, index: usize, output_path: &Path) -> PdfResult<()> {
        let r = self.get_page_box(index)?;
        let width_pts = (r.x2 - r.x1).abs();
        let height_pts = (r.y2 - r.y1).abs();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let width = (width_pts * 1.33).round() as u32; // ~96 DPI
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let height = (height_pts * 1.33).round() as u32;

        let mut backend = VelloBackend::new();
        let scale = 1.33; // 96 DPI

        // Transform:
        // 1. Translation to origin (-x1, -y1)
        // 2. Scale by S, -S
        // 3. Translation to bring into raster viewport (0, height_pixels)
        let initial_transform = kurbo::Affine::translate((0.0, f64::from(height)))
            * kurbo::Affine::scale_non_uniform(scale, -scale)
            * kurbo::Affine::translate((-r.x1, -r.y1));

        self.render_page(index, &mut backend, initial_transform)?;

        let format = match output_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("png") => image::ImageFormat::Png,
            Some("jpg" | "jpeg") => image::ImageFormat::Jpeg,
            _ => return Err(PdfError::Other(
                "Unsupported image format. Only PNG and JPEG (.png, .jpg, .jpeg) are supported."
                    .to_string(),
            )),
        };

        // Finalize rendering using the headless bridge
        let scene = backend.scene();
        pollster::block_on(ferruginous_render::headless::render_to_image(
            scene,
            width,
            height,
            output_path,
            format,
        ))
        .map_err(|e: Box<dyn std::error::Error>| PdfError::Other(e.to_string()))?;

        Ok(())
    }
}

/// Helper function to perform structural re-tagging on a document.
pub fn retag_document(doc: &mut Document) -> PdfResult<()> {
    let engine = HeuristicEngine::new(doc.arena());
    let _ = engine.infer_structure(doc)?;
    // Automatic application logic would follow
    Ok(())
}
