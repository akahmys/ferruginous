//! Document metadata and XMP processing.
//!
//! (ISO 32000-2:2020 Clause 14.3)

use xmp_writer::XmpWriter;

/// Manages XMP metadata generation and synchronization (ISO 16684-1).
pub struct XmpManager;

impl XmpManager {
    /// Generates a compliant XMP packet from the provided fields.
    ///
    /// # Arguments
    /// * `title` - The document title (dc:title).
    /// * `creator` - The document creator (dc:creator).
    /// * `description` - The document description (dc:description).
    ///
    /// # Returns
    /// A UTF-8 encoded XMP packet.
    #[must_use]
    pub fn generate_packet(
        title: Option<&str>,
        creator: Option<&str>,
        description: Option<&str>,
    ) -> Vec<u8> {
        let mut writer = XmpWriter::new();

        if let Some(t) = title {
            writer.title([(None, t)]);
        }
        if let Some(c) = creator {
            writer.creator([c]);
        }
        if let Some(d) = description {
            writer.description([(None, d)]);
        }

        writer.finish(None).into_bytes()
    }
}

/// Represents PDF Metadata (ISO 32000-2:2020 Clause 14.3).
///
/// Primarily handles XMP metadata streams.
pub struct Metadata {
    /// The metadata dictionary.
    pub dictionary: std::sync::Arc<std::collections::BTreeMap<Vec<u8>, crate::core::Object>>,
    /// The raw stream content of the metadata.
    pub stream_content: std::sync::Arc<Vec<u8>>,
}

impl Metadata {
    /// Creates a new Metadata instance.
    #[must_use]
    pub fn new(
        dictionary: std::sync::Arc<std::collections::BTreeMap<Vec<u8>, crate::core::Object>>,
        stream_content: std::sync::Arc<Vec<u8>>,
    ) -> Self {
        debug_assert!(!dictionary.is_empty()); // Rule 5: assertion density
        Self {
            dictionary,
            stream_content,
        }
    }

    /// Returns the raw XMP metadata as a UTF-8 string.
    /// Clause 14.3.2 Metadata stream
    #[must_use]
    pub fn as_xmp_string(&self) -> String {
        debug_assert!(!self.stream_content.is_empty()); // Rule 5
        String::from_utf8_lossy(&self.stream_content).into_owned()
    }

    /// Basic extraction of XMP Title (dc:title).
    #[must_use]
    pub fn title(&self) -> Option<String> {
        self.extract_xmp_value("dc:title")
    }

    /// Basic extraction of XMP Creator (dc:creator).
    #[must_use]
    pub fn creator(&self) -> Option<String> {
        self.extract_xmp_value("dc:creator")
    }

    /// Simplistic XMP value extraction without a full XML parser.
    /// (Adhering to MSRV 1.94 and minimal complexity requirements)
    fn extract_xmp_value(&self, tag: &str) -> Option<String> {
        let xmp = self.as_xmp_string();
        let start_tag = format!("<{tag}>");
        let end_tag = format!("</{tag}>");

        if let Some(start_idx) = xmp.find(&start_tag) {
            let content_start = start_idx + start_tag.len();
            if let Some(end_idx) = xmp[content_start..].find(&end_tag) {
                // Return everything between the tag, including any nested tags (e.g. rdf:Alt)
                let content = &xmp[content_start..content_start + end_idx];
                // Strip common RDF nesting if present (simplistic)
                if let Some(li_start) = content.find("<rdf:li") {
                    if let Some(li_content_start) = content[li_start..].find('>') {
                        let actual_start = li_start + li_content_start + 1;
                        if let Some(li_end) = content[actual_start..].find("</rdf:li>") {
                            return Some(content[actual_start..actual_start + li_end].trim().to_string());
                        }
                    }
                }
                return Some(content.trim().to_string());
            }
        }
        None
    }
}
