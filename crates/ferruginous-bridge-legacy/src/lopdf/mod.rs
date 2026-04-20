//! Modified lopdf core for Ferruginous Legacy Bridge.
//!
//! Original Copyright (c) 2016-2022 J-F-Liu
//! Licensed under MIT License.
//!
//! Modified for zero-copy Bytes and legacy PDF 1.7 normalization.

pub mod document;
pub mod encryption;
pub mod object;
pub mod parser;
pub mod reader;
pub mod xref;

pub use document::Document;
pub use object::{Array, Dictionary, Object, Stream};
pub use parser::Parser;
pub use reader::Reader;
pub use xref::{Xref, XrefEntry};
