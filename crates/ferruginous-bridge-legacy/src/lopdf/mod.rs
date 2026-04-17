//! Modified lopdf core for Ferruginous Legacy Bridge.
//! 
//! Original Copyright (c) 2016-2022 J-F-Liu
//! Licensed under MIT License.
//!
//! Modified for zero-copy Bytes and legacy PDF 1.7 normalization.

pub mod object;
pub mod encryption;
pub mod parser;
pub mod xref;
pub mod document;
pub mod reader;

pub use object::{Object, Dictionary, Array, Stream};
pub use parser::Parser;
pub use xref::{Xref, XrefEntry};
pub use document::Document;
pub use reader::Reader;
