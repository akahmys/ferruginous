use crate::{Document, Object};

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
        && let Some(dict) = arena.get_dict(dh) {
            if let Some(h) = arena.get_name_by_str("Title") 
                && let Some(v) = dict.get(&h)
                && let Some(s) = v.resolve(arena).as_string() {
                    info.title = Some(s);
            }
            if let Some(h) = arena.get_name_by_str("Author") 
                && let Some(v) = dict.get(&h)
                && let Some(s) = v.resolve(arena).as_string() {
                    info.author = Some(s);
            }
            if let Some(h) = arena.get_name_by_str("Subject") 
                && let Some(v) = dict.get(&h)
                && let Some(s) = v.resolve(arena).as_string() {
                    info.subject = Some(s);
            }
            if let Some(h) = arena.get_name_by_str("Creator") 
                && let Some(v) = dict.get(&h)
                && let Some(s) = v.resolve(arena).as_string() {
                    info.creator = Some(s);
            }
            if let Some(h) = arena.get_name_by_str("Producer") 
                && let Some(v) = dict.get(&h)
                && let Some(s) = v.resolve(arena).as_string() {
                    info.producer = Some(s);
            }
    }
    
    // 2. Supplement with XMP Metadata from Catalog/Metadata stream
    if let Some(Object::Dictionary(catalog_handle)) = arena.get_object(*doc.root_handle())
        && let Some(catalog_dict) = arena.get_dict(catalog_handle)
        && let Some(metadata_obj) = catalog_dict.get(&arena.name("Metadata")) {
            let resolved = metadata_obj.resolve(arena);
            if let Ok(xml_data) = doc.decode_stream(&resolved)
                && let Ok(xml_str) = std::str::from_utf8(&xml_data)
                && let Ok(xml_doc) = roxmltree::Document::parse(xml_str) {
                    apply_xmp_metadata(&xml_doc, &mut info);
            }
    }
    
    info
}

fn apply_xmp_metadata(doc: &roxmltree::Document, info: &mut MetadataInfo) {
    let dc_ns = "http://purl.org/dc/elements/1.1/";
    let xmp_ns = "http://ns.adobe.com/xap/1.0/";
    let pdf_ns = "http://ns.adobe.com/pdf/1.3/";

    // DC:Title
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((dc_ns, "title")))
         && let Some(text) = node.descendants().find(|n| n.is_text()).map(|n| n.text().unwrap_or_default()) {
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
         && let Some(text) = node.descendants().find(|n| n.is_text()).map(|n| n.text().unwrap_or_default()) {
             info.subject = Some(text.to_string());
    }
    // PDF:Keywords
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((pdf_ns, "Keywords")))
         && let Some(text) = node.text() {
             info.keywords = Some(text.to_string());
    }
    // XMP:CreatorTool
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((xmp_ns, "CreatorTool")))
         && let Some(text) = node.text() {
             info.creator = Some(text.to_string());
    }
    // PDF:Producer
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name((pdf_ns, "Producer")))
         && let Some(text) = node.text() {
             info.producer = Some(text.to_string());
    }
}
