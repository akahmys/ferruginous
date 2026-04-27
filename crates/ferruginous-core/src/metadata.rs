use crate::{Document, FromPdfObject, Object};
use std::collections::BTreeMap;

/// Refined PDF Info Dictionary (ISO 32000-2:2020 Clause 14.3.3)
#[derive(Debug, Clone, FromPdfObject)]
#[pdf_dict(clause = "14.3.3")]
pub struct PdfInfo {
    #[pdf_key("Title")]
    pub title: Option<String>,
    #[pdf_key("Author")]
    pub author: Option<String>,
    #[pdf_key("Subject")]
    pub subject: Option<String>,
    #[pdf_key("Keywords")]
    pub keywords: Option<String>,
    #[pdf_key("Creator")]
    pub creator: Option<String>,
    #[pdf_key("Producer")]
    pub producer: Option<String>,
}

/// Basic document metadata.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MetadataInfo {
    /// The document title.
    pub title: Option<String>,
    /// The document author.
    pub author: Option<String>,
    /// The document subject.
    pub subject: Option<String>,
    /// The document keywords.
    pub keywords: Option<String>,
    /// The application that created the original document.
    pub creator: Option<String>,
    /// The application that produced the PDF.
    pub producer: Option<String>,
}

pub fn extract_metadata(doc: &Document) -> MetadataInfo {
    let arena = doc.arena();
    let mut info = MetadataInfo::default();

    // 1. Extract from legacy Info dictionary
    if let Some(info_handle) = doc.info_handle()
        && let Some(obj) = arena.get_object(info_handle)
        && let Ok(pdf_info) = PdfInfo::from_pdf_object(obj, arena)
    {
        info.title = pdf_info.title;
        info.author = pdf_info.author;
        info.subject = pdf_info.subject;
        info.keywords = pdf_info.keywords;
        info.creator = pdf_info.creator;
        info.producer = pdf_info.producer;
    }

    // 2. Supplement with XMP Metadata from Catalog/Metadata stream
    if let Some(Object::Dictionary(catalog_handle)) = arena.get_object(*doc.root_handle())
        && let Some(catalog_dict) = arena.get_dict(catalog_handle)
        && let Some(metadata_obj) = catalog_dict.get(&arena.name("Metadata"))
    {
        let resolved = metadata_obj.resolve(arena);
        if let Ok(xml_data) = doc.decode_stream(&resolved)
            && let Ok(xml_str) = std::str::from_utf8(&xml_data)
            && let Ok(xml_doc) = roxmltree::Document::parse(xml_str)
        {
            apply_xmp_metadata(&xml_doc, &mut info);
        }
    }

    info
}

/// Updates the document metadata in the arena.
pub fn update_document_metadata(
    doc: &crate::Document,
    info: &MetadataInfo,
) -> crate::PdfResult<()> {
    let arena = doc.arena();

    // 1. Update legacy Info dictionary (if it exists)
    update_legacy_info(doc, info)?;

    // 2. Update XMP Metadata in Catalog
    update_xmp_metadata(doc, info)?;

    Ok(())
}

fn update_legacy_info(doc: &crate::Document, info: &MetadataInfo) -> crate::PdfResult<()> {
    let arena = doc.arena();
    if let Some(info_handle) = doc.info_handle()
        && let Some(Object::Dictionary(dh)) = arena.get_object(info_handle)
    {
        let mut dict = arena.get_dict(dh).unwrap_or_default();
        if let Some(v) = &info.title {
            dict.insert(arena.name("Title"), Object::String(v.as_bytes().to_vec().into()));
        }
        if let Some(v) = &info.author {
            dict.insert(arena.name("Author"), Object::String(v.as_bytes().to_vec().into()));
        }
        if let Some(v) = &info.subject {
            dict.insert(arena.name("Subject"), Object::String(v.as_bytes().to_vec().into()));
        }
        if let Some(v) = &info.keywords {
            dict.insert(arena.name("Keywords"), Object::String(v.as_bytes().to_vec().into()));
        }
        if let Some(v) = &info.creator {
            dict.insert(arena.name("Creator"), Object::String(v.as_bytes().to_vec().into()));
        }
        if let Some(v) = &info.producer {
            dict.insert(arena.name("Producer"), Object::String(v.as_bytes().to_vec().into()));
        }
        arena.set_dict(dh, dict);
    }
    Ok(())
}

fn update_xmp_metadata(doc: &crate::Document, info: &MetadataInfo) -> crate::PdfResult<()> {
    let arena = doc.arena();
    let root_handle = *doc.root_handle();
    if let Some(Object::Dictionary(catalog_dh)) = arena.get_object(root_handle) {
        let mut catalog_dict = arena
            .get_dict(catalog_dh)
            .ok_or_else(|| crate::error::PdfError::Other("Invalid Catalog".into()))?;

        let refined_map = build_refined_metadata_map(info);
        let xmp_str = crate::refine::metadata::info_to_xmp(&refined_map);
        let xmp_refined = crate::refine::metadata::create_metadata_stream(xmp_str);

        if let crate::refine::RefinedObject::Stream(dict, data) = xmp_refined {
            let metadata_handle = commit_metadata_stream(arena, dict, data);
            catalog_dict.insert(arena.name("Metadata"), Object::Reference(metadata_handle));
            arena.set_dict(catalog_dh, catalog_dict);
        }
    }
    Ok(())
}

fn build_refined_metadata_map(info: &MetadataInfo) -> BTreeMap<crate::object::PdfName, crate::refine::RefinedObject> {
    let mut refined_map = BTreeMap::new();
    if let Some(v) = &info.title {
        refined_map.insert(
            crate::object::PdfName::new("Title"),
            crate::refine::RefinedObject::String(v.as_bytes().to_vec().into()),
        );
    }
    if let Some(v) = &info.author {
        refined_map.insert(
            crate::object::PdfName::new("Author"),
            crate::refine::RefinedObject::String(v.as_bytes().to_vec().into()),
        );
    }
    if let Some(v) = &info.subject {
        refined_map.insert(
            crate::object::PdfName::new("Subject"),
            crate::refine::RefinedObject::String(v.as_bytes().to_vec().into()),
        );
    }
    if let Some(v) = &info.keywords {
        refined_map.insert(
            crate::object::PdfName::new("Keywords"),
            crate::refine::RefinedObject::String(v.as_bytes().to_vec().into()),
        );
    }
    if let Some(v) = &info.producer {
        refined_map.insert(
            crate::object::PdfName::new("Producer"),
            crate::refine::RefinedObject::String(v.as_bytes().to_vec().into()),
        );
    }
    refined_map
}

fn commit_metadata_stream(
    arena: &PdfArena,
    dict: BTreeMap<crate::object::PdfName, crate::refine::RefinedObject>,
    data: bytes::Bytes,
) -> Handle<Object> {
    let mut stream_dict = BTreeMap::new();
    for (k, v) in dict {
        if let crate::refine::RefinedObject::Name(n) = v {
            stream_dict.insert(arena.intern_name(k), Object::Name(arena.intern_name(n)));
        }
    }
    let sdh = arena.alloc_dict(stream_dict);
    arena.alloc_object(Object::Stream(sdh, data))
}

fn apply_xmp_metadata(doc: &roxmltree::Document, info: &mut MetadataInfo) {
    let dc_ns = "http://purl.org/dc/elements/1.1/";
    let xmp_ns = "http://ns.adobe.com/xap/1.0/";
    let pdf_ns = "http://ns.adobe.com/pdf/1.3/";

    // DC:Title
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((dc_ns, "title")))
        && let Some(text) =
            node.descendants().find(|n| n.is_text()).map(|n| n.text().unwrap_or_default())
    {
        info.title = Some(text.to_string());
    }
    // DC:Creator (Seq)
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((dc_ns, "creator"))) {
        let mut creators = Vec::new();
        for li in node.descendants().filter(|n| n.has_tag_name("li")) {
            if let Some(text) = li.text() {
                creators.push(text.to_string());
            }
        }
        if !creators.is_empty() {
            info.author = Some(creators.join(", "));
        }
    }
    // DC:Description
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((dc_ns, "description")))
        && let Some(text) =
            node.descendants().find(|n| n.is_text()).map(|n| n.text().unwrap_or_default())
    {
        info.subject = Some(text.to_string());
    }
    // PDF:Keywords
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((pdf_ns, "Keywords")))
        && let Some(text) = node.text()
    {
        info.keywords = Some(text.to_string());
    }
    // XMP:CreatorTool
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((xmp_ns, "CreatorTool")))
        && let Some(text) = node.text()
    {
        info.creator = Some(text.to_string());
    }
    // PDF:Producer
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((pdf_ns, "Producer")))
        && let Some(text) = node.text()
    {
        info.producer = Some(text.to_string());
    }
}
