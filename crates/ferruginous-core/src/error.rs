use thiserror::Error;

/// Result type for PDF operations.
pub type PdfResult<T> = Result<T, PdfError>;

/// Core error types for PDF parsing and processing.
#[derive(Debug, Error)]
pub enum PdfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Lexical error at byte {pos}: {message}")]
    Lexical {
        pos: usize,
        message: String,
    },

    #[error("Syntactic error at byte {pos}: {message}")]
    Syntactic {
        pos: usize,
        message: String,
    },

    #[error("Invalid object type: expected {expected}, found {found}")]
    InvalidType {
        expected: &'static str,
        found: &'static str,
    },

    #[error("Object not found: {0:?}")]
    ObjectNotFound(crate::types::Reference),

    #[error("Integer overflow")]
    Overflow,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Encryption not supported: {0}")]
    EncryptionNotSupported(String),

    #[error("Other error: {0}")]
    Other(String),
}
