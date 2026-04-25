//! Metadata Refinement: Conversion of /Info to XMP using xmp-writer.

use crate::object::PdfName;
use crate::refine::RefinedObject;
use bytes::Bytes;
use std::collections::BTreeMap;
use xmp_writer::XmpWriter;

/// Generates an XMP Metadata stream from a PDF Info dictionary.
pub fn info_to_xmp(info: &BTreeMap<PdfName, RefinedObject>) -> String {
    let mut writer = XmpWriter::new();

    if let Some(RefinedObject::String(s)) = info.get(&PdfName::new("Title")) {
        writer.title([(None, std::str::from_utf8(s).unwrap_or(""))]);
    }
    if let Some(RefinedObject::String(s)) = info.get(&PdfName::new("Author")) {
        writer.creator([std::str::from_utf8(s).unwrap_or("")]);
    }
    if let Some(RefinedObject::String(s)) = info.get(&PdfName::new("Subject")) {
        writer.description([(None, std::str::from_utf8(s).unwrap_or(""))]);
    }

    if let Some(RefinedObject::String(s)) = info.get(&PdfName::new("Keywords")) {
        writer.pdf_keywords(std::str::from_utf8(s).unwrap_or(""));
    }
    if let Some(RefinedObject::String(s)) = info.get(&PdfName::new("Producer")) {
        writer.producer(std::str::from_utf8(s).unwrap_or(""));
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
