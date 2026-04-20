use bytes::Bytes;
use ferruginous_doc::Document;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Deserialize, JsonSchema)]
/// Arguments for the verify_signatures tool.
pub struct VerifySignaturesArgs {
    /// Path to the PDF file to verify.
    pub path: String,
    /// Whether to attempt network-based revocation checking (default: false).
    #[serde(default)]
    pub allow_network: bool,
}

#[derive(Serialize)]
/// Represents a report for a single digital signature verification.
pub struct SignatureReport {
    /// The PDF object ID of the signature dictionary.
    pub object_id: u32,
    /// The validation status (Valid, Invalid, etc.).
    pub status: String,
    /// Additional details about the validation status.
    pub details: Option<String>,
    /// The name of the signer extracted from the signature.
    pub signer_name: Option<String>,
    /// The signing date extracted from the signature.
    pub date: Option<String>,
    /// The status of any modifications detected after signing.
    pub modification_status: String,
}

/// Implementation of the verify_signatures tool.
pub async fn verify_signatures_impl(args: VerifySignaturesArgs) -> Result<String, String> {
    let data = fs::read(&args.path).map_err(|e| format!("Failed to read file: {e}"))?;

    let doc =
        Document::open(Bytes::from(data)).map_err(|e| format!("Failed to open document: {e}"))?;

    let results =
        doc.verify_signatures().map_err(|e| format!("Signature verification error: {e}"))?;

    let mut reports = Vec::new();
    for res in results {
        let (status_str, details) = match res.status {
            ferruginous_doc::validation::ValidationStatus::Valid => ("Valid".to_string(), None),
            ferruginous_doc::validation::ValidationStatus::Invalid(msg) => {
                ("Invalid".to_string(), Some(msg))
            }
            ferruginous_doc::validation::ValidationStatus::Inconclusive(msg) => {
                ("Inconclusive".to_string(), Some(msg))
            }
            ferruginous_doc::validation::ValidationStatus::Weak(msg) => {
                ("Weak Integrity".to_string(), Some(msg))
            }
            ferruginous_doc::validation::ValidationStatus::Untrusted(msg) => {
                ("Untrusted".to_string(), Some(msg))
            }
            ferruginous_doc::validation::ValidationStatus::Revoked(msg) => {
                ("Revoked".to_string(), Some(msg))
            }
        };

        let mdp_str = match res.mdp_status {
            ferruginous_doc::MdpStatus::NoModifications => {
                "No modifications after signing".to_string()
            }
            ferruginous_doc::MdpStatus::AllowedModifications => {
                "Allowed modifications only".to_string()
            }
            ferruginous_doc::MdpStatus::DisallowedModifications(msg) => {
                format!("DISALLOWED modifications: {msg}")
            }
            ferruginous_doc::MdpStatus::NotSignatoryRevision => {
                "Not the signatory's revision".to_string()
            }
        };

        reports.push(SignatureReport {
            object_id: res.signature_id,
            status: status_str,
            details,
            signer_name: res.name,
            date: res.date,
            modification_status: mdp_str,
        });
    }

    serde_json::to_string_pretty(&reports).map_err(|e| format!("Serialization error: {e}"))
}
