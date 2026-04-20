//! Ferruginous Doc: Document Structure and Resource Management.
//!
//! (ISO 32000-2:2020 Clause 7.5 and 7.7)

pub mod conformance;
pub mod document;
pub mod font;
pub mod legacy;
pub mod page;
pub mod security;
pub mod signature;
pub mod validation;
pub mod xref;

pub use document::{Document, MdpStatus, SignatureVerificationResult};
pub use font::{FontDescriptor, FontResource};
pub use page::{Page, PageTree};
pub use signature::Signature;
pub use validation::{SignatureVerifier, ValidationStatus};
pub use xref::{XRefEntry, XRefIndex};
