//! Global document compliance validation (PDF/A, PDF/UA, Tagged PDF).
//!
//! (ISO 32000-2:2020 Clause 14 & ISO 19005-4)

use crate::core::{PdfResult, Resolver};
use crate::loader::PdfDocument;
use crate::font::Font;
use crate::catalog::Catalog;
use crate::structure::TaggedPdfValidator;
use std::collections::BTreeSet;

/// Represents the results of a compliance check.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// List of errors found.
    pub errors: Vec<String>,
    /// List of warnings found.
    pub warnings: Vec<String>,
}

impl ValidationReport {
    /// Returns true if the document is strictly compliant (no errors).
    pub fn is_compliant(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Primary validator for document-level compliance.
pub struct ComplianceValidator<'a> {
    doc: &'a PdfDocument,
}

impl<'a> ComplianceValidator<'a> {
    /// Creates a new validator for the given document.
    pub fn new(doc: &'a PdfDocument) -> Self {
        Self { doc }
    }

    /// Performs full compliance validation (Unicode, Tagged PDF, Metadata).
    pub fn validate_all(&self) -> PdfResult<ValidationReport> {
        let mut report = ValidationReport {
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        self.validate_unicode(&mut report)?;
        self.validate_tagged_pdf(&mut report)?;
        self.validate_metadata(&mut report)?;
        self.validate_associated_files(&mut report)?;

        Ok(report)
    }

    /// Validates Unicode integrity for all fonts (ISO 19005-4 / PDF/A-4).
    pub fn validate_unicode(&self, report: &mut ValidationReport) -> PdfResult<()> {
        let resolver = self.doc.resolver();
        let pages = self.doc.page_tree()?;
        let mut validated_fonts = BTreeSet::new();

        for i in 0..pages.get_count() {
            let page = pages.get_page(i)?;
            let resources = match page.resources() {
                Some(r) => r,
                None => continue,
            };
            
            // Note: Resources can be inherited, but page.resources() handles that.
            if let Some(fonts_dict) = resources.dictionary.get(b"Font".as_ref()) {
                let fonts_dict = resolver.resolve_if_ref(fonts_dict)?.as_dict_arc()
                    .ok_or_else(|| crate::core::PdfError::InvalidType { expected: "Dictionary".into(), found: "Font Resources".into() })?;

                for (name, font_obj) in fonts_dict.iter() {
                    let font_ref = match font_obj {
                        crate::core::Object::Reference(r) => *r,
                        _ => continue, // Simple fonts might not be references, but rare for global resources
                    };

                    if validated_fonts.contains(&font_ref) { continue; }
                    validated_fonts.insert(font_ref);

                    let font_dict = resolver.resolve(&font_ref)?.as_dict_arc()
                        .ok_or_else(|| crate::core::PdfError::InvalidType { expected: "Dictionary".into(), found: "Font Resource".into() })?;
                    
                    let font = Font::from_dict(&font_dict, &resolver)?;
                    
                    // PDF/A-4 Clause 6.2.11.7: Every font shall have a ToUnicode CMap or 
                    // a predefined CMap that maps to Unicode.
                    if font.to_unicode.is_none() {
                        let has_unicode_enc = font.encoding_cmap.as_ref()
                            .is_some_and(|m| m.name.contains("UniJIS") || m.name.contains("UTF"));
                        
                        if !has_unicode_enc && !font.is_multi_byte() {
                            // Simple fonts Check (BaseEncoding)
                            if font.base_encoding.is_none() && font.differences.is_empty() {
                                report.errors.push(format!(
                                    "Font \"{}\" (Ref {:?}) lacks Unicode mapping (ISO 19005-4:2020 6.2.11.7)",
                                    String::from_utf8_lossy(name), font_ref
                                ));
                            }
                        } else if !has_unicode_enc {
                            report.errors.push(format!(
                                "CIDFont \"{}\" (Ref {:?}) lacks ToUnicode CMap (ISO 19005-4:2020 6.2.11.8)",
                                String::from_utf8_lossy(name), font_ref
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validates Tagged PDF requirements (ISO 32000-2:2020 Clause 14.8).
    pub fn validate_tagged_pdf(&self, report: &mut ValidationReport) -> PdfResult<()> {
        let catalog = self.doc.catalog()?;
        
        // Check MarkInfo (ISO 32000-2 Clause 14.7.1)
        let mark_info = catalog.dictionary.get(b"MarkInfo".as_slice())
            .and_then(|o| catalog.resolver.resolve_if_ref(o).ok())
            .and_then(|o| o.as_dict_arc());
        
        let is_marked = mark_info.and_then(|d| d.get(b"Marked".as_slice()).and_then(|o| o.as_bool()))
            .unwrap_or(false);

        if let Some(struct_root_obj) = catalog.dictionary.get(b"StructTreeRoot".as_slice()) {
            let struct_root = catalog.resolver.resolve_if_ref(struct_root_obj)?.as_dict_arc()
                .ok_or_else(|| crate::core::PdfError::InvalidType { expected: "Dictionary".into(), found: "/StructTreeRoot".into() })?;

            let logic = crate::structure::LogicalStructure::new(struct_root, catalog.resolver);
            let validator = TaggedPdfValidator::new_owned(logic);
            
            for err in validator.validate() {
                report.errors.push(format!("Tagged PDF: {err}"));
            }

            if !is_marked {
                report.warnings.push("Document has StructTreeRoot but /MarkInfo /Marked is false (Clause 14.8)".into());
            }
        } else if is_marked {
            report.errors.push("Catalog has /MarkInfo /Marked true but lacks /StructTreeRoot".into());
        }

        Ok(())
    }

    /// Validates XMP Metadata compliance (ISO 16684-1).
    pub fn validate_metadata(&self, report: &mut ValidationReport) -> PdfResult<()> {
        let catalog = self.doc.catalog()?;
        
        if let Some(meta_obj) = catalog.dictionary.get(b"Metadata".as_slice()) {
            let meta_ref = match meta_obj {
                crate::core::Object::Reference(r) => *r,
                _ => return Ok(()), // Should be an indirect reference
            };

            let meta_stream = catalog.resolver.resolve(&meta_ref)?;
            if let crate::core::Object::Stream(dict, data) = meta_stream {
                 // Check for /Type /Metadata and /Subtype /XML
                 if dict.get(b"Type".as_slice()).and_then(|o| o.as_str()) != Some(b"Metadata") {
                     report.warnings.push(format!("Metadata stream at {:?} missing /Type /Metadata", meta_ref));
                 }
                 
                 // Basic XMP presence check
                 let decoded = crate::filter::decode_stream(&dict, &data).unwrap_or(data.to_vec());
                 let decoded_str = String::from_utf8_lossy(&decoded);
                 if !decoded_str.starts_with("<?xpacket") && !decoded_str.contains("<x:xmpmeta") {
                     report.errors.push(format!("Metadata stream at {:?} is not a valid XMP packet", meta_ref));
                 }
            }
        } else {
            report.warnings.push("Document lacks XMP metadata (Required for PDF/A)".into());
        }

        Ok(())
    }

    /// Validates Associated Files (AF) compliance (ISO 32000-2 Clause 14.13).
    pub fn validate_associated_files(&self, report: &mut ValidationReport) -> PdfResult<()> {
        let catalog = self.doc.catalog()?;
        let af_refs = catalog.associated_files();
        
        for af_ref in af_refs {
            let af_obj = catalog.resolver.resolve(&af_ref)?;
            if let Some(dict) = af_obj.as_dict() {
                // ISO 32000-2 Clause 14.13.1: AFRelationship key is required
                if !dict.contains_key(b"AFRelationship".as_ref()) {
                    report.errors.push(format!("Associated File spec at {:?} missing required /AFRelationship key", af_ref));
                }
            }
        }

        Ok(())
    }
}
