# Ferruginous Rebirth: Clean-Slate Reconstruction

Scraping the current prototype and rebuilding the PDF 2.0 toolkit with a cleaner, more robust architecture.

## User Review Required

> [!IMPORTANT]
> This plan completely replaces the existing codebase. The current code has been moved to `.legacy/` for reference. We will not "port" code line-by-line, but rather rewrite it using the mature insights (Zero-copy, CID/WMode fixes) from Phase 19.

> [!WARNING]
> The initial focus will be on the core SDK and rendering foundation. UI features like `ferruginous-ui` will be reconstructed later once the core is stable.

## Proposed Changes

### [ferruginous-core] [NEW]
The base layer for PDF type systems and low-level parsing.

#### [NEW] [types.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-core/src/types.rs)
- Definition of `Object`, `Reference`, and `Name` types.
- Strict use of `bytes::Bytes` for raw data to ensure zero-copy.
- `Resolver` trait definition.

#### [NEW] [lexer.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-core/src/lexer.rs) / [parser.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-core/src/parser.rs)
- Modernized lexer using `Peekable<I>` for optimized tokenization.
- Recursive descent parser with strict ISO 32000-2 compliance.

### [ferruginous-doc] [NEW]
Document structure and resource management.

#### [NEW] [xref.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-doc/src/xref.rs)
- Unified XRef handler (Cross-reference tables and streams).
- Robust recovery logic for "dirty" PDFs.

#### [NEW] [page.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-doc/src/page.rs)
- Efficient Page Tree resolution.
- Inheritance-aware resource dictionary management.

### [ferruginous-render] [NEW]
The graphics engine and rendering abstraction.

#### [NEW] [bridge.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-render/src/bridge.rs)
- `RenderBackend` trait (the Render Bridge).
- Headless rendering specialization for JPEG/PNG output (using Vello + `image` crate).

#### [NEW] [text.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-render/src/text.rs)
- Integrated Multibyte/Vertical text layout engine (Japanese support).

### [ferruginous-mcp] [NEW]
The interface layer for AI-driven document manipulation.

#### [NEW] [server.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-mcp/src/server.rs)
- Implementation of the Model Context Protocol (using `rmcp`).
- **Read/Edit Tools**: Full CRUD capability for PDF data.
- **Visual Tools**: Capability to trigger headless rendering and return image paths for AI inspection.

---

## Open Questions

- **UI Phase**: Confirming that `ferruginous-ui` (egui-based) will be deferred until the core engine and headless verification are stable.

- **Feature Parity**: Which features from Phase 19 (e.g., Encryption, Signatures) are high priority for the Rebirth?

## Verification Plan

### Automated Tests
- `cargo test` across all new crates.
- **SSoT Verification**: Use the [PDF Association Sample Suite](https://github.com/pdf-association/pdf-issues) as the primary verification source.
- `pdf-spec-mcp` validation for all new parsing logic against ISO 32000-2.

### Manual Verification
- Rendering a set of "insight-heavy" PDFs (Japanese vertical text, transparency groups) once the bridge is ready.
