//! Core PDF primitives, coordinate transformations, and error types.

/// ISO 32000-2:2020 Clause 7.3 - General Object Types.
pub mod types;
/// Reliable Rust-15 (RR-15) compliant error handling.
pub mod error;

pub use types::{Object, Reference, Resolver};
pub use error::{PdfError, PdfResult, ParseErrorVariant, StructureErrorVariant, ContentErrorVariant, ValidationErrorVariant};
