use crate::{McpError, McpResult};
use bytes::Bytes;
use ferruginous_core::{Object, PdfError, PdfName, PdfResult, Reference, Resolver};
use ferruginous_doc::{Document, PageTree};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;

/// Arguments for the structural audit tool.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct AuditArgs {
    /// Path to the PDF document to audit.
    pub path: String,
}

/// A summary report of the structural audit.
#[derive(Serialize, Deserialize, Debug)]
pub struct AuditReport {
    /// Overall audit status (PASSED, FAILED, CRITICAL_FAILURE).
    pub status: String,
    /// Detailed list of findings during the audit.
    pub findings: Vec<Finding>,
}

/// A specific observation or error found during the audit.
#[derive(Serialize, Deserialize, Debug)]
pub struct Finding {
    /// Severity level (Info, Warning, Error).
    pub severity: String,
    /// The functional area where the issue was found (Structural, Compliance).
    pub category: String,
    /// Detailed description of the finding.
    pub message: String,
}

/// Implementation of the structural audit logic.
pub async fn audit_document_impl(args: AuditArgs) -> Result<String, String> {
    audit_document_internal(args).map_err(|e| e.to_string())
}

fn audit_document_internal(args: AuditArgs) -> McpResult<String> {
    let mut findings = Vec::new();

    // 1. Load File
    let data = Bytes::from(
        fs::read(&args.path).map_err(|e| McpError::Other(format!("Failed to read file: {e}")))?,
    );

    // 2. Open Document (Structural Check)
    let doc: Document = match Document::open(data) {
        Ok(d) => d,
        Err(e) => {
            findings.push(Finding {
                severity: "Error".into(),
                category: "Structural".into(),
                message: format!("Document failed to open: {e}"),
            });
            return Ok(serde_json::to_string_pretty(&AuditReport {
                status: "CRITICAL_FAILURE".into(),
                findings,
            })?);
        }
    };

    findings.push(Finding {
        severity: "Info".into(),
        category: "Structural".into(),
        message: "XRef chain and trailer resolved successfully.".into(),
    });

    let root_ref = doc.root();
    validate_catalog(&doc, &root_ref, &mut findings);
    validate_page_tree(&doc, &root_ref, &mut findings);
    validate_compliance(&doc, &mut findings);

    let status = if findings.iter().any(|f| f.severity == "Error") { "FAILED" } else { "PASSED" };

    Ok(serde_json::to_string_pretty(&AuditReport { status: status.into(), findings })?)
}

fn validate_catalog(doc: &Document, root_ref: &Reference, findings: &mut Vec<Finding>) {
    match Resolver::resolve(doc, root_ref) {
        Ok(obj) => {
            if let Some(dict) = obj.as_dict() {
                let type_key = PdfName::from("Type");
                let type_val = dict.get(&type_key).and_then(|o| o.as_name());
                if type_val.map(|n| n.0.as_ref()) != Some(b"Catalog") {
                    findings.push(Finding {
                        severity: "Warning".into(),
                        category: "Compliance".into(),
                        message: "Root object missing /Type /Catalog (Required for ISO 32000)"
                            .into(),
                    });
                }
                let pages_key = PdfName::from("Pages");
                if dict.get(&pages_key).is_none() {
                    findings.push(Finding {
                        severity: "Error".into(),
                        category: "Compliance".into(),
                        message: "Catalog missing required /Pages key".into(),
                    });
                }
            } else {
                findings.push(Finding {
                    severity: "Error".into(),
                    category: "Structural".into(),
                    message: "Root object is not a dictionary".into(),
                });
            }
        }
        Err(e) => {
            findings.push(Finding {
                severity: "Error".into(),
                category: "Structural".into(),
                message: format!("Failed to resolve Root object: {e}"),
            });
        }
    }
}

fn validate_page_tree(doc: &Document, root_ref: &Reference, findings: &mut Vec<Finding>) {
    let res: PdfResult<()> = (|| {
        let catalog = Resolver::resolve(doc, root_ref)?;
        let dict = catalog
            .as_dict()
            .ok_or_else(|| PdfError::Other("Catalog is not a dictionary".into()))?;

        let pages_key = PdfName::from("Pages");
        if let Some(Object::Reference(pages_ref)) = dict.get(&pages_key) {
            let tree = PageTree::new(*pages_ref, doc);
            match tree.count() {
                Ok(count) => {
                    findings.push(Finding {
                        severity: "Info".into(),
                        category: "Structural".into(),
                        message: format!("Page tree contains {count} pages."),
                    });
                    check_page_range(&tree, count, findings);
                }
                Err(e) => {
                    findings.push(Finding {
                        severity: "Error".into(),
                        category: "Structural".into(),
                        message: format!("Failed to walk page tree: {e}"),
                    });
                }
            }
        } else {
            findings.push(Finding {
                severity: "Error".into(),
                category: "Compliance".into(),
                message: "Catalog missing required /Pages reference".into(),
            });
        }
        Ok(())
    })();

    if let Err(e) = res {
        findings.push(Finding {
            severity: "Error".into(),
            category: "Structural".into(),
            message: format!("Structural validation aborted: {e}"),
        });
    }
}

fn check_page_range(tree: &PageTree, count: usize, findings: &mut Vec<Finding>) {
    if count > 0 {
        if let Err(e) = tree.page(0) {
            findings.push(Finding {
                severity: "Error".into(),
                category: "Structural".into(),
                message: format!("Failed to resolve first page: {e}"),
            });
        }
        if count > 1 {
            if let Err(e) = tree.page(count - 1) {
                findings.push(Finding {
                    severity: "Error".into(),
                    category: "Structural".into(),
                    message: format!("Failed to resolve last page: {e}"),
                });
            }
        }
    }
}

fn validate_compliance(doc: &Document, findings: &mut Vec<Finding>) {
    match doc.compliance_info() {
        Ok(info) => {
            // 1. PDF/A Checks
            if let Some(part) = info.metadata.pdf_a_part {
                findings.push(Finding {
                    severity: "Info".into(),
                    category: "Compliance".into(),
                    message: format!("Document claims PDF/A conformance (Part {part})."),
                });

                if part == 4 && info.output_intents.is_empty() {
                    findings.push(Finding {
                        severity: "Warning".into(),
                        category: "Compliance".into(),
                        message: "PDF/A-4 usually requires at least one OutputIntent for color management.".into(),
                    });
                }
            }

            // 2. PDF/X Checks
            if let Some(version) = &info.metadata.pdf_x_version {
                findings.push(Finding {
                    severity: "Info".into(),
                    category: "Compliance".into(),
                    message: format!("Document claims PDF/X conformance ({version})."),
                });
            }

            // 3. PDF/UA (Accessibility) Checks
            if let Some(part) = info.metadata.pdf_ua_part {
                findings.push(Finding {
                    severity: "Info".into(),
                    category: "Compliance".into(),
                    message: format!("Document claims PDF/UA conformance (Part {part})."),
                });

                if !info.has_struct_tree {
                    findings.push(Finding {
                        severity: "Error".into(),
                        category: "Compliance".into(),
                        message:
                            "PDF/UA compliant files MUST have a /StructTreeRoot in the Catalog."
                                .into(),
                    });
                }

                if !info.is_marked {
                    findings.push(Finding {
                        severity: "Error".into(),
                        category: "Compliance".into(),
                        message: "PDF/UA compliant files MUST have /MarkInfo << /Marked true >> in the Catalog.".into(),
                    });
                }
            } else if info.has_struct_tree {
                findings.push(Finding {
                    severity: "Info".into(),
                    category: "Compliance".into(),
                    message:
                        "Document contains logical structure (/StructTreeRoot) but no PDF/UA claim."
                            .into(),
                });
            }
        }
        Err(e) => {
            findings.push(Finding {
                severity: "Warning".into(),
                category: "Compliance".into(),
                message: format!("Compliance metadata extraction failed or missing: {e}"),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_simple_pdf() {
        let sample_path = "../../samples/standard/Simple PDF 2.0 file.pdf";
        let args = AuditArgs { path: sample_path.into() };
        let result = audit_document_impl(args).await.unwrap();
        println!("Audit Result:\n{result}");
        assert!(result.contains("\"status\": \"PASSED\""));
    }
}
