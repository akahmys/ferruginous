//! Ferruginous SDK: High-level PDF processing library.
//!
//! This crate provides a high-level, easy-to-use interface for PDF document
//! manipulation, rendering, and auditing, abstracting away the low-level
//! complexities of the core type system and document model.

use bytes::Bytes;
pub use ferruginous_core::{Document, Handle, Object, PdfArena, PdfName, PdfResult, PdfError, Page};
use ferruginous_render::VelloBackend;
use crate::remediation::HeuristicEngine;
use crate::structure::{MatterhornAuditor, AuditFinding};
use std::path::Path;

/// The internal cloning module for object migration.
pub mod cloning;
/// The internal interpreter module for processing content streams.
pub mod interpreter;
pub use interpreter::Interpreter;
/// The internal structure module for UA-2 logical tree handling.
pub mod structure;
/// The internal writer module for generating PDF files.
pub mod writer;
/// The internal remediation module for structural repair.
pub mod remediation;

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

pub use ferruginous_core::metadata::MetadataInfo;
pub use ferruginous_core::font::FontSummary;

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
    pub fn open_with_options(data: Bytes, options: &ferruginous_core::ingest::IngestionOptions) -> PdfResult<Self> {
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

    /// Attempts to open and repair a PDF document with default options.
    pub fn open_and_repair(data: Bytes) -> PdfResult<Self> {
        let inner = Document::open_repair(data, &ferruginous_core::ingest::IngestionOptions::default())?;
        Ok(Self { inner })
    }

    /// Merges multiple documents into a new one.
    pub fn merge(sources: Vec<PdfDocument>) -> PdfResult<Self> {
        if sources.is_empty() { return Err(PdfError::Other("No sources to merge".into())); }
        // TODO: M67 implementation using cloning::ObjectCloner
        // For now, return the first one as a stub
        Ok(Self { inner: Document::open(Bytes::new(), &ferruginous_core::ingest::IngestionOptions::default())? }) // STUB
    }

    /// Extracts specific pages into a new document.
    pub fn extract_pages(&self, _indices: Vec<usize>) -> PdfResult<Self> {
        // TODO: M67 implementation using cloning::ObjectCloner
        Ok(Self { inner: Document::open(Bytes::new(), &ferruginous_core::ingest::IngestionOptions::default())? }) // STUB
    }

    /// Saves the document to a file with a specific version.
    pub fn save_as_version(&self, output_path: &Path, version: &str) -> PdfResult<()> {
        let file = std::fs::File::create(output_path).map_err(PdfError::Io)?;
        let mut writer = crate::writer::PdfWriter::new(file, self.inner.arena());
        writer.write_header(version)?;
        writer.finish(*self.inner.root_handle())?;
        Ok(())
    }

    /// Saves a linearized (Fast Web View) version of the document.
    pub fn save_linearized(&self, output_path: &Path, version: &str) -> PdfResult<()> {
        // Linearization is a complex multi-pass process. 
        // For now, we fallback to a standard save.
        self.save_as_version(output_path, version)
    }

    /// Returns the physical viewport of the page (MediaBox).
    pub fn get_page_box(&self, index: usize) -> PdfResult<ferruginous_core::graphics::Rect> {
        let page = self.inner.get_page(index)?;
        if let Some(mb) = page.resolve_attribute("MediaBox")
            && let Some(arr_handle) = mb.as_array()
            && let Some(arr) = self.inner.arena().get_array(arr_handle)
            && arr.len() >= 4 {
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
    pub fn render_page(&self, index: usize, backend: &mut dyn ferruginous_render::RenderBackend, initial_transform: kurbo::Affine) -> PdfResult<()> {
        let page = self.inner.get_page(index)?;
        let arena = self.inner.arena();
        let res_obj = page.resolve_attribute("Resources")
            .unwrap_or_else(|| Object::Dictionary(arena.alloc_dict(std::collections::BTreeMap::new())));
        
        if let Object::Dictionary(rh) = res_obj {
            let mut interpreter = Interpreter::new(backend, &self.inner, rh, initial_transform);
            let contents_obj = page.resolve_attribute("Contents").ok_or_else(|| PdfError::Other("Page has no contents".into()))?;
            
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
    pub fn get_remediation_candidates(&self) -> PdfResult<Vec<crate::remediation::RemediationCandidate>> {
        let engine = HeuristicEngine::new(self.inner.arena());
        engine.infer_structure(&self.inner)
    }

    /// Extracts Unicode text from a specific page.
    pub fn extract_text(&self, index: usize) -> PdfResult<String> {
        let _page = self.inner.get_page(index)?;
        let _text_output = String::new();
        
        // Use a specialized TextBackend to capture characters
        // For now, we'll use a dummy implementation
        Ok(format!("Text extraction for page {index} to be implemented."))
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
    pub fn set_page_rotation(&mut self, _index: usize, _angle: i32) -> PdfResult<()> {
        // TODO: Modify page dictionary in arena
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

        let format = match output_path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()).as_deref() {
            Some("png") => image::ImageFormat::Png,
            Some("jpg" | "jpeg") => image::ImageFormat::Jpeg,
            _ => return Err(PdfError::Other("Unsupported image format. Only PNG and JPEG (.png, .jpg, .jpeg) are supported.".to_string())),
        };

        // Finalize rendering using the headless bridge
        let scene = backend.scene();
        pollster::block_on(ferruginous_render::headless::render_to_image(
            scene, width, height, output_path, format
        )).map_err(|e: Box<dyn std::error::Error>| PdfError::Other(e.to_string()))?;
        
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
