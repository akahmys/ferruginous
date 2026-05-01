//! Metadata Refinement: Conversion of /Info to XMP using xmp-writer.

use crate::object::PdfName;
use crate::refine::RefinedObject;
use bytes::Bytes;
use std::collections::BTreeMap;
use xmp_writer::XmpWriter;

/// Generates an XMP Metadata stream from a PDF Info dictionary.
pub fn info_to_xmp(info: &BTreeMap<PdfName, RefinedObject>) -> String {
    let mut writer = XmpWriter::new();

    if let Some(obj) = info.get(&PdfName::new("Title")) {
        let val = match obj {
            RefinedObject::Text(s) => s.clone(),
            RefinedObject::String(s) | RefinedObject::Hex(s) => {
                crate::refine::text::recover_string(s)
            }
            _ => "".into(),
        };
        if !val.is_empty() {
            writer.title([(None, val.as_str())]);
        }
    }
    if let Some(obj) = info.get(&PdfName::new("Author")) {
        let val = match obj {
            RefinedObject::Text(s) => s.clone(),
            RefinedObject::String(s) | RefinedObject::Hex(s) => {
                crate::refine::text::recover_string(s)
            }
            _ => "".into(),
        };
        if !val.is_empty() {
            writer.creator([val.as_str()]);
        }
    }
    if let Some(obj) = info.get(&PdfName::new("Subject")) {
        let val = match obj {
            RefinedObject::Text(s) => s.clone(),
            RefinedObject::String(s) | RefinedObject::Hex(s) => {
                crate::refine::text::recover_string(s)
            }
            _ => "".into(),
        };
        if !val.is_empty() {
            writer.description([(None, val.as_str())]);
        }
    }

    if let Some(obj) = info.get(&PdfName::new("Keywords")) {
        let val = match obj {
            RefinedObject::Text(s) => s.clone(),
            RefinedObject::String(s) | RefinedObject::Hex(s) => {
                crate::refine::text::recover_string(s)
            }
            _ => "".into(),
        };
        if !val.is_empty() {
            writer.pdf_keywords(val.as_str());
        }
    }
    if let Some(obj) = info.get(&PdfName::new("Producer")) {
        let val = match obj {
            RefinedObject::Text(s) => s.clone(),
            RefinedObject::String(s) | RefinedObject::Hex(s) => {
                crate::refine::text::recover_string(s)
            }
            _ => "".into(),
        };
        if !val.is_empty() {
            writer.producer(val.as_str());
        }
    }

    writer.finish(None)
}

/// Creates a RefinedObject representing the Metadata stream.
pub fn create_metadata_stream(xmp: String) -> RefinedObject {
    let mut dict = BTreeMap::new();
    dict.insert(PdfName::new("Type"), RefinedObject::Name(PdfName::new("Metadata")));
    dict.insert(PdfName::new("Subtype"), RefinedObject::Name(PdfName::new("XML")));

    RefinedObject::Stream(dict, Bytes::from(xmp))
}
