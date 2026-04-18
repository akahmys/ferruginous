//! Ferruginous Doc: Document Structure and Resource Management.
//!
//! (ISO 32000-2:2020 Clause 7.5 and 7.7)

pub mod document;
pub mod page;
pub mod xref;
pub mod font;
pub mod legacy;
pub mod security;
pub mod signature;
pub mod validation;

pub use document::{Document, SignatureVerificationResult, MdpStatus};
pub use page::{Page, PageTree};
pub use xref::{XRefEntry, XRefIndex};
pub use font::{FontResource, FontDescriptor};
pub use signature::Signature;
pub use validation::{SignatureVerifier, ValidationStatus};
