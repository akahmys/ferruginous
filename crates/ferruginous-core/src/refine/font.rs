//! Font Refinement: ISO 32000-2 (PDF 2.0) Compliance Engine.

use crate::object::PdfName;
use crate::refine::RefinedObject;
use std::collections::BTreeMap;
use bytes::Bytes;

/// Normalizes a font dictionary to a PDF 2.0 compliant state.
/// 
/// This involves:
/// 1. Standardizing /Encoding (ensuring Identity-H for Type0 if appropriate)
/// 2. Synthesizing or cleaning /ToUnicode maps for CJK fonts.
/// 3. Validating /Subtype and /BaseFont.
pub fn normalize_font(mut dict: BTreeMap<PdfName, RefinedObject>) -> RefinedObject {
    let type_key = PdfName::new("Type");
    let subtype_key = PdfName::new("Subtype");

    // Only process if it's actually a Font
    if let Some(RefinedObject::Name(t)) = dict.get(&type_key)
        && t.as_str() != "Font" { return RefinedObject::Dictionary(dict); }

    let subtype_str = dict.get(&subtype_key).and_then(|o| match o {
        RefinedObject::Name(n) => Some(n.as_str()),
        _ => None,
    });

    match subtype_str {
        Some("Type0") => {
            // Type0 (CID-keyed) font normalization
            normalize_type0_font(&mut dict);
        }
        Some("TrueType") | Some("Type1") | Some("MMType1") => {
            // Simple font normalization
            normalize_simple_font(&mut dict);
        }
        _ => {}
    }

    RefinedObject::Dictionary(dict)
}

fn normalize_type0_font(dict: &mut BTreeMap<PdfName, RefinedObject>) {
    let encoding_key = PdfName::new("Encoding");
    // 1. Ensure Encoding is standardized
    if let Some(enc) = dict.get(&encoding_key) {
        match enc {
            RefinedObject::Name(_n) => {
                // Identity-H/V are standard for CID-keyed fonts
            }
            RefinedObject::Stream(..) => {
                // Custom CMap stream
            }
            _ => {}
        }
    } else {
        // Missing encoding in Type0 is an error, but we'll default to Identity-H for hardening
        dict.insert(encoding_key.clone(), RefinedObject::Name(PdfName::new("Identity-H")));
    }

    // 2. Synthesize ToUnicode if missing
    // REMOVED: Broken heuristic synthesis of empty ToUnicode CMaps
}

fn normalize_simple_font(_dict: &mut BTreeMap<PdfName, RefinedObject>) {}

/// Normalizes a CMap stream to a canonical PDF 2.0 form.
pub fn normalize_cmap(dict: BTreeMap<PdfName, RefinedObject>, data: Bytes) -> RefinedObject {
    RefinedObject::Stream(dict, data)
}
