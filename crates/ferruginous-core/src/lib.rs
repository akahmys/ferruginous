//! Ferruginous Core: PDF Type System and Lexical Analysis.
//!
//! This crate provides the foundational types and low-level parsing logic
//! for ISO 32000-2:2020 (PDF 2.0).

pub mod error;
pub mod lexer;
pub mod parser;
pub mod types;
pub mod graphics;
pub mod filters;

pub use error::{PdfError, PdfResult};
pub use graphics::{Color, Matrix, Rect, GraphicsState, BlendMode};
pub use parser::Parser;
pub use types::{Object, PdfName, Reference, Resolver};
