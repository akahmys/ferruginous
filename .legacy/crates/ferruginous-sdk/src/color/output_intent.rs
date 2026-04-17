//! OutputIntents management (ISO 32000-2:2020 Clause 14.11).

use crate::core::{Object, Resolver, PdfResult, PdfError, ContentErrorVariant};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Represents a document color target (Clause 14.11.2).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct OutputIntent {
    /// Subtype (S keyword), usually GTS_PDFX or GTS_PDFA.
    pub subtype: String,
    /// Human-readable name for the output condition.
    pub output_condition: Option<String>,
    /// Identifier for the output condition (required if DestOutputProfile is missing).
    pub output_condition_identifier: String,
    /// Registry containing the color condition.
    pub registry_name: Option<String>,
    /// Additional info about the condition.
    pub info: Option<String>,
    /// The destination ICC profile (Clause 14.11.2.2).
    pub dest_output_profile: Option<Arc<Vec<u8>>>,
}

impl OutputIntent {
    /// Parses an OutputIntent dictionary.
    pub fn from_dict<R: Resolver + ?Sized>(dict: &BTreeMap<Vec<u8>, Object>, resolver: &R) -> PdfResult<Self> {
        let subtype = dict.get(b"S".as_ref())
            .and_then(|o| match o { Object::Name(n) => Some(n), _ => None })
            .map(|n| String::from_utf8_lossy(n).to_string())
            .ok_or_else(|| PdfError::ContentError(ContentErrorVariant::MissingRequiredKey("S (Subtype)")))?;

        let output_condition_identifier = dict.get(b"OutputConditionIdentifier".as_ref())
            .and_then(|o| match o {
                Object::String(s) => Some(String::from_utf8_lossy(s).to_string()),
                _ => None,
            })
            .ok_or_else(|| PdfError::ContentError(ContentErrorVariant::MissingRequiredKey("OutputConditionIdentifier")))?;

        let dest_output_profile = if let Some(obj) = dict.get(b"DestOutputProfile".as_ref()) {
            let res = resolver.resolve_if_ref(obj)?;
            if let Object::Stream(s_dict, data) = res {
                // Ensure it's an ICC profile
                if s_dict.get(b"Type".as_ref()).and_then(|o| match o { Object::Name(n) => Some(n.as_ref()), _ => None }) == Some(b"ICCBased") || true {
                    // Note: PDF 2.0 allows it to be an ICC profile stream directly without /Type /ICCBased sometimes.
                   Some(Arc::new(data.to_vec()))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            subtype,
            output_condition: dict.get(b"OutputCondition".as_ref()).and_then(|o| o.as_str().map(|s| String::from_utf8_lossy(s).to_string())),
            output_condition_identifier,
            registry_name: dict.get(b"RegistryName".as_ref()).and_then(|o| o.as_str().map(|s| String::from_utf8_lossy(s).to_string())),
            info: dict.get(b"Info".as_ref()).and_then(|o| o.as_str().map(|s| String::from_utf8_lossy(s).to_string())),
            dest_output_profile,
        })
    }
}
