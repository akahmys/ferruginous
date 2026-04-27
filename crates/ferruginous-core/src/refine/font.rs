use crate::object::PdfName;
use super::RefinedObject;
use crate::font::FontResource;
use bytes::Bytes;
use std::collections::BTreeMap;

/// Normalizes a font dictionary to a canonical PDF 2.0 form.
pub fn normalize_font(
    mut dict: BTreeMap<PdfName, RefinedObject>,
    resource: Option<&FontResource>,
) -> RefinedObject {
    let type_key = PdfName::new("Type");
    let subtype_key = PdfName::new("Subtype");

    // Only process if it's actually a Font
    if let Some(RefinedObject::Name(t)) = dict.get(&type_key)
        && t.as_str() != "Font" {
        return RefinedObject::Dictionary(dict);
    }

    let type0_name = PdfName::new("Type0");
    let cid2_name = PdfName::new("CIDFontType2");

    // Decide whether to use Identity-H normalization
    // We only do this if we have a reliable GID map OR it's already a CID font.
    let _has_gid_map = resource.map(|r| !r.unicode_to_gid.is_empty()).unwrap_or(false);
    let is_embedded = resource.map(|r| r.data.is_some()).unwrap_or(false);
    
    if let Some(RefinedObject::Name(st)) = dict.get(&subtype_key) {
        if st == &type0_name && is_embedded {
            normalize_type0_font(&mut dict, resource);
        } else if st == &cid2_name {
            // HARDENING: Inject /CIDToGIDMap /Identity directly into the CIDFont
            dict.insert(PdfName::new("CIDToGIDMap"), RefinedObject::Name(PdfName::new("Identity")));
        }
    }

    RefinedObject::Dictionary(dict)
}

fn normalize_type0_font(dict: &mut BTreeMap<PdfName, RefinedObject>, resource: Option<&FontResource>) {
    // 1. Set Encoding to Identity-H/V based on WMode
    let encoding_name = if resource.map(|r| r.wmode == 1).unwrap_or(false) {
        "Identity-V"
    } else {
        "Identity-H"
    };
    dict.insert(PdfName::new("Encoding"), RefinedObject::Name(PdfName::new(encoding_name)));
    
    // 2. Inject GID-based ToUnicode CMap to ensure searchability
    if let Some(unicode_bytes) = resource.and_then(|res| res.generate_standard_tounicode()) {
        let mut uni_dict = BTreeMap::new();
        uni_dict.insert(PdfName::new("Length"), RefinedObject::Integer(unicode_bytes.len() as i64));
        dict.insert(PdfName::new("ToUnicode"), RefinedObject::Stream(uni_dict, Bytes::from(unicode_bytes)));
    }
}

/// Normalizes a CMap stream to a canonical PDF 2.0 form.
pub fn normalize_cmap(dict: BTreeMap<PdfName, RefinedObject>, data: Bytes) -> RefinedObject {
    RefinedObject::Stream(dict, data)
}
