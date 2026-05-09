use super::RefinedObject;
use crate::font::FontResource;
use crate::object::PdfName;
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
        && t.as_str() != "Font"
    {
        return RefinedObject::Dictionary(dict);
    }

    let subtype = dict.get(&subtype_key).and_then(|o| o.as_str()).map(|s| s.to_string());
    let is_embedded = resource.map(|r| r.data.is_some()).unwrap_or(false);

    if let Some(st_str) = subtype {
        if st_str == "Type0" && is_embedded {
            normalize_type0_font(&mut dict, resource);
        }

        // CIDFonts (descendants) need CIDToGIDMap Identity if missing
        if st_str == "CIDFontType0" || st_str == "CIDFontType2" {
            dict.entry(PdfName::new("CIDToGIDMap")).or_insert_with(|| RefinedObject::Name(PdfName::new("Identity")));
        }
    }

    RefinedObject::Dictionary(dict)
}

fn normalize_type0_font(
    _dict: &mut BTreeMap<PdfName, RefinedObject>,
    _resource: Option<&FontResource>,
) {
    // HARDENING: Do NOT override the original Encoding unless we are 100% sure we can restructure the stream.
    /*
    let encoding_name = if resource.map(|r| r.wmode == 1).unwrap_or(false) {
        "Identity-V"
    } else {
        "Identity-H"
    };
    dict.insert(PdfName::new("Encoding"), RefinedObject::Name(PdfName::new(encoding_name)));
    */

    // HARDENING: Do NOT inject inline streams into the dictionary.
    // PDF 1.7+ requires ToUnicode to be an indirect object.
    /*
    if let Some(unicode_bytes) = resource.and_then(|res| res.generate_standard_tounicode()) {
        let mut uni_dict = BTreeMap::new();
        uni_dict.insert(PdfName::new("Length"), RefinedObject::Integer(unicode_bytes.len() as i64));
        dict.insert(PdfName::new("ToUnicode"), RefinedObject::Stream(uni_dict, Bytes::from(unicode_bytes)));
    }
    */
}

/// Normalizes a CMap stream to a canonical PDF 2.0 form.
pub fn normalize_cmap(dict: BTreeMap<PdfName, RefinedObject>, data: Bytes) -> RefinedObject {
    RefinedObject::Stream(dict, data)
}
