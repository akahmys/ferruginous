//! Ferruginous PDF Engine SDK.
//!
//! A high-integrity PDF parsing and manipulation library designed for
//! Reliable Rust-15 (RR-15) compliance and ISO 32000-2:2020 adherence.

/// Core PDF types, matrices, and error handling.
pub mod core;
pub mod forms;
pub mod ocg;
pub mod arlington;
pub mod catalog;
/// PDF content stream parsing and processing (ISO 32000-2:2020 Clause 7.8).
pub mod content;
pub mod metadata;
pub mod colorspace;
pub mod font;
pub mod graphics;
pub mod lexer;
pub mod loader;
/// Page tree and page leaf node management.
pub mod page;
/// Page tree resolution and inheritance.
pub mod resolver;
/// Resource dictionary management.
pub mod resources;
/// Text-specific state and operators.
pub mod text;
pub mod trailer;
pub mod xref;
pub mod navigation;
pub mod annotation;
pub mod structure;
pub mod filter;
pub mod security;
pub mod signature;
pub mod redaction;
pub mod cmap;
/// Physical PDF object serialization and file writing.
pub mod serialize;
/// Text layer extraction and management.
pub mod text_layer;
/// Full-text search engine for PDF content.
pub mod search;
/// Document-level editing and incremental updates.
pub mod editor;
/// Multimedia and 3D support (ISO 32000-2:2020 Clause 13).
pub mod multimedia;

/// Advanced shading tessellation and mesh subdivision (ISO 32000-2:2020 Clause 8.7.4.5).
pub mod shading_tess;

pub use loader::PdfDocument;
pub use editor::PdfEditor;
pub use page::{Page, PageTree};
pub use graphics::{GraphicsState, Color, DrawOp, GlyphInstance};
pub use serialize as writer; // Backwards compatibility if needed, or just use serialize.
pub use core::{Object, Reference, Resolver, PdfError, PdfResult};
