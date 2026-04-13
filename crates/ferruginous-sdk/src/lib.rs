//! Ferruginous PDF Engine SDK.
//!
//! A high-integrity PDF parsing and manipulation library designed for
//! Reliable Rust-15 (RR-15) compliance and ISO 32000-2:2020 adherence.

/// Core PDF types, matrices, and error handling.
pub mod core;
/// Interactive Forms (AcroForm) management (Clause 12.7).
pub mod forms;
/// Optional Content Groups (Layers) management (Clause 8.11).
pub mod ocg;
/// Arlington PDF Model integration and validation.
pub mod arlington;
/// Document Catalog and root structure resolution (Clause 7.7.2).
pub mod catalog;
/// PDF content stream parsing and processing (ISO 32000-2:2020 Clause 7.8).
pub mod content;
/// Document metadata and XMP support (Clause 14.3).
pub mod metadata;
/// Color spaces and ICC profile support (Clause 8.6).
pub mod colorspace;
/// Font dictionary and glyph mapping (Clause 9).
pub mod font;
/// Graphics state and drawing primitives (Clause 8.4).
pub mod graphics;
/// Low-level PDF tokenization and parsing.
pub mod lexer;
/// Document loading and version detection.
pub mod loader;
/// Page tree and page leaf node management.
pub mod page;
/// Page tree resolution and inheritance.
pub mod resolver;
/// Resource dictionary management.
pub mod resources;
/// Text-specific state and operators.
pub mod text;
/// Trailer dictionary and document termination (Clause 7.5.5).
pub mod trailer;
/// Cross-reference table and stream management (Clause 7.5.4).
pub mod xref;
/// Navigation and outline management (Clause 12.3).
pub mod navigation;
/// Annotation and widget management (Clause 12.5).
pub mod annotation;
/// Logical Structure and Tagged PDF support (Clause 14.7).
pub mod structure;
/// Stream filters (Flate, DCT, etc.) (Clause 7.4).
pub mod filter;
/// Encryption and security handlers (Clause 7.6).
pub mod security;
/// Digital signatures and Cryptographic verification (Clause 12.8).
pub mod signature;
/// Redaction and content removal (Clause 12.5.6.23).
pub mod redaction;
/// ToUnicode CMaps and character mapping (Clause 9.10).
pub mod cmap;
/// Adobe-Japan1 CMap resources and character mapping.
pub mod cmap_aj1;
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
/// Standard font encodings.
pub mod encoding;

/// Advanced shading tessellation and mesh subdivision (ISO 32000-2:2020 Clause 8.7.4.5).
pub mod shading_tess;

pub use loader::PdfDocument;
pub use editor::PdfEditor;
pub use page::{Page, PageTree};
pub use graphics::{GraphicsState, Color, DrawOp, GlyphInstance};
pub use serialize as writer; // Backwards compatibility if needed, or just use serialize.
pub use core::{Object, Reference, Resolver, PdfError, PdfResult};
