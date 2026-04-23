//! Ferruginous Core: PDF 2.0 Refinery Engine.
//!
//! (ISO 32000-2:2020 Compliance Engine v2.1)
//!
//! This crate provides the high-performance Arena-based object model
//! and the Ingestion Gateway for the Ferruginous toolkit.

pub mod arena;
pub mod document;
pub mod filters;
pub mod graphics;
pub mod handle;
pub mod ingest;
pub mod object;
pub mod lexer;
pub mod parser;
pub mod font;
pub mod refine;
pub mod color;
pub mod metadata;

extern crate chardetng;

pub use arena::{PdfArena, RemappingTable};
pub use document::Document;
pub use document::page::Page;
pub use crate::refine::{ParallelRefinery, commit_to_arena};

pub use handle::Handle;
pub use object::{Object, PdfName, Reference};
pub use ingest::LopdfIngestor;
pub use graphics::{Color, Matrix, BlendMode, LineCap, LineJoin, WindingRule, StrokeStyle, PixelFormat};

pub use error::PdfError;
/// Standard Result type for Ferruginous Core operations.
pub type PdfResult<T> = Result<T, PdfError>;

pub mod error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum PdfError {
        #[error("IO error: {0}")]
        Io(#[from] std::io::Error),
        #[error("Parse error: {0}")]
        Parse(String),
        #[error("Ingestion error: {0}")]
        Ingestion(String),
        #[error("Arena error: {0}")]
        Arena(String),
        #[error("Filter error: {0}")]
        Filter(String),
        #[error("Lopdf error: {0}")]
        Lopdf(#[from] lopdf::Error),
        #[error("Other error: {0}")]
        Other(String),
    }
}
