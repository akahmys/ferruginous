use ferruginous_core::{Object, PdfError, PdfName, PdfResult, Reference, Resolver};
use roxmltree::Document as XmlDocument;
use std::collections::BTreeMap;

/// Represents the conformance metadata of a PDF document.
#[derive(Debug, Clone, Default)]
pub struct ConformanceMetadata {
    pub pdf_a_part: Option<u32>,
    pub pdf_a_conformance: Option<String>,
    pub pdf_x_version: Option<String>,
    pub pdf_ua_part: Option<u32>,
}

impl ConformanceMetadata {
    /// Parses XMP metadata stream to extract conformance information.
    pub fn from_xmp(data: &[u8]) -> PdfResult<Self> {
        let xml_str = std::str::from_utf8(data)
            .map_err(|_| PdfError::Other("Invalid UTF-8 in XMP metadata".into()))?;

        let doc = XmlDocument::parse(xml_str)
            .map_err(|e| PdfError::Other(format!("XMP XML parse error: {}", e)))?;

        let mut meta = ConformanceMetadata::default();

        // Namespaces
        let pdfaid_ns = "http://www.aiim.org/pdfa/ns/id/";
        let pdfuaid_ns = "http://www.aiim.org/pdfua/ns/id/";
        let _pdfx_ns = "http://ns.adobe.com/pdfx/1.3/"; // Common PDF/X ns

        for node in doc.descendants() {
            if node.tag_name().namespace() == Some(pdfaid_ns) {
                match node.tag_name().name() {
                    "part" => meta.pdf_a_part = node.text().and_then(|t| t.trim().parse().ok()),
                    "conformance" => {
                        meta.pdf_a_conformance = node.text().map(|s| s.trim().to_string())
                    }
                    _ => {}
                }
            } else if node.tag_name().namespace() == Some(pdfuaid_ns) {
                if node.tag_name().name() == "part" {
                    meta.pdf_ua_part = node.text().and_then(|t| t.trim().parse().ok());
                }
            } else if node.tag_name().name() == "GTS_PDFXVersion" {
                meta.pdf_x_version = node.text().map(|s| s.to_string());
            }
        }

        Ok(meta)
    }
}

/// Represents a PDF OutputIntent.
#[derive(Debug, Clone)]
pub struct OutputIntent {
    pub subtype: String,
    pub output_condition_identifier: String,
    pub info: Option<String>,
    pub registry_name: Option<String>,
    pub destination_profile_ref: Option<Reference>,
}

impl OutputIntent {
    pub fn from_dict(dict: &BTreeMap<PdfName, Object>) -> PdfResult<Self> {
        let subtype = dict
            .get(&"S".into())
            .and_then(|o| o.as_name())
            .map(|n| String::from_utf8_lossy(n.as_ref()).into_owned())
            .ok_or_else(|| PdfError::Other("Missing /S in OutputIntent".into()))?;

        let output_condition_identifier = dict
            .get(&"OutputConditionIdentifier".into())
            .and_then(|o| o.as_string())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .ok_or_else(|| {
                PdfError::Other("Missing /OutputConditionIdentifier in OutputIntent".into())
            })?;

        let info = dict
            .get(&"Info".into())
            .and_then(|o| o.as_string())
            .map(|s| String::from_utf8_lossy(s).into_owned());

        let registry_name = dict
            .get(&"RegistryName".into())
            .and_then(|o| o.as_string())
            .map(|s| String::from_utf8_lossy(s).into_owned());

        let destination_profile_ref =
            dict.get(&"DestOutputProfile".into()).and_then(|o| o.as_reference());

        Ok(Self {
            subtype,
            output_condition_identifier,
            info,
            registry_name,
            destination_profile_ref,
        })
    }
}

/// Level of severity for an audit issue.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A specific issue found during a compliance audit.
#[derive(Debug, Clone)]
pub struct AuditIssue {
    pub standard: String,
    pub severity: Severity,
    pub message: String,
}

/// High-level compliance information for a document.
#[derive(Debug, Clone)]
pub struct ComplianceInfo {
    pub metadata: ConformanceMetadata,
    pub output_intents: Vec<OutputIntent>,
    pub has_struct_tree: bool,
    pub is_marked: bool,
    pub issues: Vec<AuditIssue>,
}

impl ComplianceInfo {
    pub fn extract(
        doc: &dyn Resolver,
        catalog_ref: &Reference,
        header_version: &str,
    ) -> PdfResult<Self> {
        let catalog_obj = doc.resolve(catalog_ref)?;
        let catalog =
            catalog_obj.as_dict().ok_or_else(|| PdfError::Other("Invalid catalog".into()))?;

        // 1. Metadata
        let mut metadata = ConformanceMetadata::default();
        let mut has_metadata_stream = false;
        if let Some(meta_ref) = catalog.get(&"Metadata".into()).and_then(|o| o.as_reference())
            && let Ok(Object::Stream(_, data)) = doc.resolve(&meta_ref)
        {
            metadata = ConformanceMetadata::from_xmp(&data).unwrap_or_default();
            has_metadata_stream = true;
        }

        // 2. OutputIntents
        let mut output_intents = Vec::new();
        if let Some(intents) = catalog.get(&"OutputIntents".into()).and_then(|o| o.as_array()) {
            for intent_obj in intents.iter() {
                if let Some(r) = intent_obj.as_reference()
                    && let Some(dict) = doc.resolve(&r)?.as_dict()
                    && let Ok(oi) = OutputIntent::from_dict(dict)
                {
                    output_intents.push(oi);
                } else if let Some(dict) = intent_obj.as_dict()
                    && let Ok(oi) = OutputIntent::from_dict(dict)
                {
                    output_intents.push(oi);
                }
            }
        }

        // 3. Structure Tree
        let has_struct_tree = catalog.contains_key(&"StructTreeRoot".into());

        // 4. MarkInfo
        let is_marked = catalog
            .get(&"MarkInfo".into())
            .and_then(|o| o.as_dict())
            .and_then(|d| d.get(&"Marked".into()))
            .and_then(|o| o.as_bool())
            .unwrap_or(false);

        // 5. Audit Issues
        let mut issues = Vec::new();

        // ISO 32000-2 (PDF 2.0) Audit
        if header_version == "2.0" {
            if !has_metadata_stream {
                issues.push(AuditIssue {
                    standard: "ISO 32000-2".into(),
                    severity: Severity::Error,
                    message: "Missing /Metadata stream (mandatory for PDF 2.0)".into(),
                });
            }
            if catalog.get(&"Version".into()).and_then(|o| o.as_name()).map(|n| n.as_ref())
                != Some(b"2.0")
            {
                issues.push(AuditIssue {
                    standard: "ISO 32000-2".into(),
                    severity: Severity::Warning,
                    message: "Catalog /Version entry should be /2.0 for PDF 2.0 documents".into(),
                });
            }
        }

        // PDF/UA-2 Pre-check
        if !has_struct_tree {
            issues.push(AuditIssue {
                standard: "PDF/UA-2".into(),
                severity: Severity::Error,
                message: "Missing Structure Tree (required for Accessibility)".into(),
            });
        }
        if !is_marked {
            issues.push(AuditIssue {
                standard: "PDF/UA-2".into(),
                severity: Severity::Error,
                message: "Document is not flagged as /Marked".into(),
            });
        }

        // PDF/A-4 Pre-check
        if output_intents.is_empty() {
            issues.push(AuditIssue {
                standard: "PDF/A-4".into(),
                severity: Severity::Warning,
                message: "Missing OutputIntents (ICC profile recommended for archiving)".into(),
            });
        }

        Ok(Self { metadata, output_intents, has_struct_tree, is_marked, issues })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xmp() {
        let xmp = r#"
            <?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
            <x:xmpmeta xmlns:x="adobe:ns:meta/">
              <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                <rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
                  <pdfaid:part>4</pdfaid:part>
                  <pdfaid:conformance>F</pdfaid:conformance>
                </rdf:Description>
                <rdf:Description rdf:about="" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/">
                  <pdfuaid:part>2</pdfuaid:part>
                </rdf:Description>
                <rdf:Description rdf:about="" xmlns:pdfx="http://ns.adobe.com/pdfx/1.3/">
                  <pdfx:GTS_PDFXVersion>PDF/X-6</pdfx:GTS_PDFXVersion>
                </rdf:Description>
              </rdf:RDF>
            </x:xmpmeta>
            <?xpacket end="w"?>
        "#;

        let meta = ConformanceMetadata::from_xmp(xmp.as_bytes()).unwrap();
        assert_eq!(meta.pdf_a_part, Some(4));
        assert_eq!(meta.pdf_a_conformance.as_deref(), Some("F"));
        assert_eq!(meta.pdf_ua_part, Some(2));
        assert_eq!(meta.pdf_x_version.as_deref(), Some("PDF/X-6"));
    }
}
