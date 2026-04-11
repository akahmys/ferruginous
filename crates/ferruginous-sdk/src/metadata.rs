//! Document metadata and XMP processing.
//! (ISO 32000-2:2020 Clause 14.3)


/// Represents PDF Metadata (ISO 32000-2:2020 Clause 14.3).
/// Primarily handles XMP metadata streams.
pub struct Metadata {
    /// The metadata dictionary.
    pub dictionary: std::sync::Arc<std::collections::BTreeMap<Vec<u8>, crate::core::Object>>,
    /// The raw stream content of the metadata.
    pub stream_content: std::sync::Arc<Vec<u8>>,
}

impl Metadata {
    /// Creates a new Metadata instance.
    #[must_use] pub fn new(dictionary: std::sync::Arc<std::collections::BTreeMap<Vec<u8>, crate::core::Object>>, stream_content: std::sync::Arc<Vec<u8>>) -> Self {
        debug_assert!(!dictionary.is_empty()); // Rule 5: assertion density
        // Type check is usually /Metadata but check spec for requirements
        Self { dictionary, stream_content }
    }

    /// Returns the raw XMP metadata as a UTF-8 string.
    /// Clause 14.3.2 Metadata stream
    #[must_use] pub fn as_xmp_string(&self) -> String {
        debug_assert!(!self.stream_content.is_empty()); // Rule 5
        String::from_utf8_lossy(&self.stream_content).into_owned()
    }

    /// Basic extraction of XMP Title (dc:title).
    #[must_use] pub fn title(&self) -> Option<String> {
        self.extract_xmp_value("dc:title")
    }

    /// Basic extraction of XMP Creator (dc:creator).
    #[must_use] pub fn creator(&self) -> Option<String> {
        self.extract_xmp_value("dc:creator")
    }

    /// Simplistic XMP value extraction without a full XML parser.
    /// (Adhering to MSRV 1.85.0 and minimal complexity requirements)
    fn extract_xmp_value(&self, tag: &str) -> Option<String> {
        let xmp = self.as_xmp_string();
        let start_tag = format!("<{tag}>");
        let end_tag = format!("</{tag}>");

        if let Some(start_idx) = xmp.find(&start_tag) {
            let content_start = start_idx + start_tag.len();
            if let Some(end_idx) = xmp[content_start..].find(&end_tag) {
                // Return everything between the tag, including any nested tags (e.g. rdf:Alt)
                return Some(xmp[content_start..content_start + end_idx].trim().to_string());
            }
        }
        None
    }
}
