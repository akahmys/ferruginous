use crate::{Document, Object};
use std::collections::BTreeMap;

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
    if let Some(Object::Dictionary(dh)) = doc.info_handle().and_then(|h| arena.get_object(h))
        && let Some(dict) = arena.get_dict(dh)
    {
        if let Some(h) = arena.get_name_by_str("Title")
            && let Some(v) = dict.get(&h)
            && let Some(s) = v.resolve(arena).as_string()
        {
            info.title = Some(s);
        }
        if let Some(h) = arena.get_name_by_str("Author")
            && let Some(v) = dict.get(&h)
            && let Some(s) = v.resolve(arena).as_string()
        {
            info.author = Some(s);
        }
        if let Some(h) = arena.get_name_by_str("Subject")
            && let Some(v) = dict.get(&h)
            && let Some(s) = v.resolve(arena).as_string()
        {
            info.subject = Some(s);
        }
        if let Some(h) = arena.get_name_by_str("Creator")
            && let Some(v) = dict.get(&h)
            && let Some(s) = v.resolve(arena).as_string()
        {
            info.creator = Some(s);
        }
        if let Some(h) = arena.get_name_by_str("Producer")
            && let Some(v) = dict.get(&h)
            && let Some(s) = v.resolve(arena).as_string()
        {
            info.producer = Some(s);
        }
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

    // 2. Update XMP Metadata in Catalog
    let root_handle = *doc.root_handle();
    if let Some(Object::Dictionary(catalog_dh)) = arena.get_object(root_handle) {
        let mut catalog_dict = arena
            .get_dict(catalog_dh)
            .ok_or_else(|| crate::error::PdfError::Other("Invalid Catalog".into()))?;

        // Convert MetadataInfo back to a refined map for info_to_xmp
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

        let xmp_str = crate::refine::metadata::info_to_xmp(&refined_map);
        let xmp_refined = crate::refine::metadata::create_metadata_stream(xmp_str);

        // We need to commit this refined object to the arena.
        // Since we don't have a direct "commit" here without a remapping table,
        // we'll implement a simplified version.
        if let crate::refine::RefinedObject::Stream(dict, data) = xmp_refined {
            let mut stream_dict = BTreeMap::new();
            for (k, v) in dict {
                if let crate::refine::RefinedObject::Name(n) = v {
                    stream_dict.insert(arena.intern_name(k), Object::Name(arena.intern_name(n)));
                }
            }
            let sdh = arena.alloc_dict(stream_dict);
            let metadata_stream = Object::Stream(sdh, data);
            let metadata_handle = arena.alloc_object(metadata_stream);

            catalog_dict.insert(arena.name("Metadata"), Object::Reference(metadata_handle));
            arena.set_dict(catalog_dh, catalog_dict);
        }
    }

    Ok(())
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
