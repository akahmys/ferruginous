//! PDF SDK Error types.
//!
//! (RR-15 Rule 11: Explicit Error)

use thiserror::Error;
use crate::core::Reference;

/// Detailed variants for parsing errors to avoid String-based error messages.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseErrorVariant {
    /// Unexpected end of file at the specified offset.
    #[error("Unexpected end of file at offset {offset}")]
    UnexpectedEof { 
        /// The byte offset where EOF was encountered.
        offset: u64 
    },
    /// Invalid object identifier or reference format.
    #[error("Invalid object ID at offset {offset}")]
    InvalidObjectId { 
        /// The byte offset where the invalid ID was found.
        offset: u64 
    },
    /// Object generation number mismatch.
    #[error("Generation mismatch for object {id}: expected {expected}, found {found}")]
    GenerationMismatch { 
        /// The object ID.
        id: u32, 
        /// The expected generation number from XRef.
        expected: u16, 
        /// The generation number found in the object header.
        found: u16 
    },
    /// Offset is too large or points beyond the file end.
    #[error("Offset {offset} too large or beyond EOF")]
    InvalidOffset { 
        /// The invalid byte offset.
        offset: usize 
    },
    /// Error encountered during PDF header parsing.
    #[error("Header parse error at {offset}: {details}")]
    HeaderError { 
        /// The byte offset of the header.
        offset: u64, 
        /// Detailed description of the header error.
        details: String 
    },
    /// Exceeded the maximum allowed filter nesting depth.
    #[error("Too many filter layers: {found} (limit: {limit})")]
    ExcessiveFilterLayers { 
        /// Number of filter layers found.
        found: usize, 
        /// The defined nesting limit.
        limit: usize 
    },
    /// The specified filter is not supported by this engine.
    #[error("Unsupported filter: {filter} at offset {offset}")]
    UnsupportedFilter { 
        /// The name of the unsupported filter.
        filter: String, 
        /// The byte offset where the filter was specified.
        offset: u64 
    },
    /// Invalid PNG predictor type.
    #[error("Invalid PNG filter type: {filter_type} at offset {offset}")]
    InvalidPngFilter { 
        /// The invalid filter type number.
        filter_type: u8, 
        /// The byte offset.
        offset: u64 
    },
    /// Error during hexadecimal decoding.
    #[error("Hex decode error at offset {offset}: {details}")]
    HexDecodeError { 
        /// The byte offset where decoding failed.
        offset: u64, 
        /// Detailed decode error message.
        details: String 
    },
    /// Error within a literal string (e.g., unbalanced parentheses).
    #[error("Literal string error at offset {offset}: {details}")]
    LiteralStringError { 
        /// The byte offset of the string.
        offset: u64, 
        /// Detailed error message.
        details: String 
    },
    /// Stream data length does not match the /Length key.
    #[error("Stream length mismatch: expected {expected}, found {found} at offset {offset}")]
    StreamLengthMismatch { 
        /// The length specified in the dictionary.
        expected: usize, 
        /// The actual byte length of the stream data.
        found: usize, 
        /// The start offset of the stream.
        offset: u64 
    },
    /// Generic error during CMap parsing.
    #[error("CMap parse error: {0}")]
    CMapError(String),
    /// Generic error during compressed object stream parsing.
    #[error("Compressed object parse error: {0}")]
    CompressedObjectError(String),
    /// Generic/fallback parse error.
    #[error("General parse error at offset {offset}: {details}")]
    General { 
        /// The byte offset where the error occurred.
        offset: u64, 
        /// Detailed error message.
        details: String 
    },
}

impl ParseErrorVariant {
    /// Creates a general PDF error at the given offset.
    pub fn general(offset: u64, details: impl Into<String>) -> Self {
        Self::General { offset, details: details.into() }
    }
}

impl From<String> for ParseErrorVariant {
    fn from(s: String) -> Self {
        ParseErrorVariant::General { offset: 0, details: s }
    }
}

impl From<&str> for ParseErrorVariant {
    fn from(s: &str) -> Self {
        ParseErrorVariant::General { offset: 0, details: s.to_string() }
    }
}

/// Detailed variants for PDF structure errors (trailers, root, etc).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StructureErrorVariant {
    /// Missing or invalid /Root entry in the trailer dictionary.
    #[error("Missing required key /Root in trailer")]
    MissingRoot,
    /// Cross-reference table or stream not found at the expected offset.
    #[error("No cross-reference information found at offset {0}")]
    NoXRefFound(usize),
    /// Too many cross-reference updates (potential circular chain).
    #[error("Too many /Prev chains (possible circular reference)")]
    TooManyXRefLayers,
    /// The trailer object has an invalid type or structure.
    #[error("Invalid trailer type")]
    InvalidTrailerType,
    /// General format error in the PDF logical structure.
    #[error("Invalid structure or object format")]
    InvalidFormat,
    /// Invalid value for the /Prev key in a cross-reference stream.
    #[error("Invalid /Prev value")]
    InvalidPrev,
    /// Malformed cross-reference table.
    #[error("Invalid XRef table format at offset {offset}: {details}")]
    InvalidXRefFormat { 
        /// Offset where the invalid XRef was found.
        offset: u64, 
        /// Details of the structural error.
        details: String 
    },
    /// Circular reference detected during indirect object resolution.
    #[error("Circular reference detected for object {0:?}")]
    CircularReference(Reference),
    /// A required key is missing from a dictionary within a specific context.
    #[error("Missing required key {key} in {context}")]
    MissingRequiredKey { 
        /// The name of the missing key.
        key: String, 
        /// The context (e.g. "Page dictionary") where it was expected.
        context: String 
    },
}

/// Detailed variants for content stream and page structure errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContentErrorVariant {
    /// An object (e.g. Page) is missing a required dictionary key.
    #[error("Page missing required key: {0}")]
    MissingRequiredKey(&'static str),
    /// The given object reference is of an unsupported or invalid type.
    #[error("Unsupported content type: {0}")]
    UnsupportedType(String),
    /// Resource not found (e.g. missing font or XObject).
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    /// Generic content interpretation error.
    #[error("General content error: {0}")]
    General(String),
    /// The document uses a color space not yet supported by this engine.
    #[error("Unsupported color space: {0}")]
    UnsupportedColorSpace(String),
}

impl From<String> for ContentErrorVariant {
    fn from(s: String) -> Self {
        ContentErrorVariant::General(s)
    }
}

impl From<&str> for ContentErrorVariant {
    fn from(s: &str) -> Self {
        ContentErrorVariant::General(s.to_string())
    }
}

/// Detailed variants for validation errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationErrorVariant {
    /// Failure against the Arlington PDF Model schema.
    #[error("Arlington validation failed: {0:?}")]
    Arlington(Vec<String>),
    /// General document-level validation error.
    #[error("General validation error: {0}")]
    General(String),
}

impl From<String> for ValidationErrorVariant {
    fn from(s: String) -> Self {
        ValidationErrorVariant::General(s)
    }
}

impl From<&str> for ValidationErrorVariant {
    fn from(s: &str) -> Self {
        ValidationErrorVariant::General(s.to_string())
    }
}

/// The primary error type for the Ferruginous PDF engine.
#[derive(Debug, Error)]
pub enum PdfError {
    /// The requested indirect object was not found.
    #[error("Object not found: {0:?}")]
    ObjectNotFound(Reference),
    /// The object exists but has a different type than requested.
    #[error("Invalid object type: expected {expected}, found {found}")]
    InvalidType { 
        /// The expected type string.
        expected: String, 
        /// The actual type found.
        found: String 
    },
    /// Low-level syntax or data parsing error.
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseErrorVariant),
    /// Higher-level structural or logical PDF error.
    #[error("Structure error: {0}")]
    StructureError(#[from] StructureErrorVariant),
    /// Error related to document resources (fonts, images, color spaces).
    #[error("Resource error: {0}")]
    ResourceError(String),
    /// Page content or direct interpretation error.
    #[error("Content error: {0}")]
    ContentError(#[from] ContentErrorVariant),
    /// Standard I/O error from the underlying filesystem or stream.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    /// Schema or specification validation failure.
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationErrorVariant),
    /// Encryption, permission, or signature trust error.
    #[error("Security error: {0}")]
    SecurityError(String),
    /// Error during document serialization or writing.
    #[error("Writer error: {0}")]
    WriterError(String),
}

impl Clone for PdfError {
    fn clone(&self) -> Self {
        match self {
            Self::ObjectNotFound(r) => Self::ObjectNotFound(*r),
            Self::InvalidType { expected, found } => Self::InvalidType { expected: expected.clone(), found: found.clone() },
            Self::ParseError(e) => Self::ParseError(e.clone()),
            Self::StructureError(e) => Self::StructureError(e.clone()),
            Self::ResourceError(s) => Self::ResourceError(s.clone()),
            Self::ContentError(e) => Self::ContentError(e.clone()),
            Self::IoError(e) => Self::IoError(std::io::Error::new(e.kind(), e.to_string())),
            Self::Validation(e) => Self::Validation(e.clone()),
            Self::SecurityError(s) => Self::SecurityError(s.clone()),
            Self::WriterError(s) => Self::WriterError(s.clone()),
        }
    }
}

impl PartialEq for PdfError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ObjectNotFound(a), Self::ObjectNotFound(b)) => a == b,
            (Self::InvalidType { expected: a1, found: a2 }, Self::InvalidType { expected: b1, found: b2 }) => a1 == b1 && a2 == b2,
            (Self::ParseError(a), Self::ParseError(b)) => a == b,
            (Self::StructureError(a), Self::StructureError(b)) => a == b,
            (Self::ResourceError(a), Self::ResourceError(b)) => a == b,
            (Self::ContentError(a), Self::ContentError(b)) => a == b,
            (Self::IoError(a), Self::IoError(b)) => a.kind() == b.kind(),
            (Self::Validation(a), Self::Validation(b)) => a == b,
            (Self::SecurityError(a), Self::SecurityError(b)) => a == b,
            (Self::WriterError(a), Self::WriterError(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for PdfError {}

/// Unified result type for the SDK.
pub type PdfResult<T> = Result<T, PdfError>;
