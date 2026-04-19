# Ferruginous Development Reflections & Post-Mortems

This document tracks development friction, failures, and subsequent protocol improvements.

---

## [2026-04-18] Phase 7: Transparency & Blend Modes

### 1. Phenomenon: Multiple Iterative Fixes
- **Observation**: Required 7 build-fix cycles to reach a green state for unit tests.
- **Cause**: 
    - `RenderBackend` trait was modified without updating `MockBackend` and `headless.rs` implementers in the same step.
    - Missing imports (`BlendMode`, `Arc`) in `lib.rs` after automated content replacement.
- **Protocol Feedback**: Added **HDD Clause 5: Atomic Interface Compliance**. Modification of shared traits must now be accompanied by immediate implementation updates across all crates.

### 2. Phenomenon: Integration Test Failure (Missing Resolver)
- **Observation**: `rendering_test.rs` failed with `Other("No resolver for font resolution")`.
- **Cause**: The `Interpreter` was instantiated in `PdfDocument` without the document's resolver and page-level resources.
- **Protocol Feedback**: Added **RR-15 Clause 16: Context Propagation Guard**. High-level interpreters must now enforce mandatory environment injection via type-level constraints.

### 3. Phenomenon: Type Mismatches in Automated Edits
- **Observation**: `PdfName` vs `&str` and `Bytes` conversion issues during `gs` operator implementation.
- **Cause**: Over-reliance on `.into()` without explicit namespacing.
- **Protocol Feedback**: Updated **HDD Section 5** to emphasize explicit type naming in cross-crate interactions.
---

## [2026-04-18] Phase 8: Text & Fonts

### 1. Phenomenon: CMap Parser Fragility
- **Observation**: Initial parsing logic failed on multi-line `codespacerange` and PostScript-style comments.
- **Cause**: Attempted to use line-based string split instead of a proper token stream.
- **Protocol Feedback**: Added **HDD Clause 6: Token-First Parsing for DSLs**. Any non-trivial domain language (CMap, Type 1 charstrings) MUST use a token-based lexer to handle whitespace and nesting correctly.

### 2. Phenomenon: Static Registry Efficiency
- **Observation**: Resolving CIDFonts and Standard 14 fonts dynamically through dictionaries created complex borrow-checker lifetimes.
- **Cause**: High-frequency lookup of standard resources within the rendering loop.
- **Protocol Feedback**: Added **RR-15 Clause 17: Immutable Static Registry Pattern**. Standard resources (CMap templates, Font Metrics) should be formalized into thread-safe static registries using `OnceLock` to minimize runtime overhead and borrow-check friction.

### 3. Phenomenon: BBox Calculation Precision
- **Observation**: Initial text bounds were overly simplified, leading to clipping in vertical scripts.
- **Cause**: Ignored font's explicit ascent/descent metrics in favor of just horizontal advance.
- **Protocol Feedback**: Updated **HDD Section 5** to mandate "Metric-Aware Bounds" for all layout-critical content types (Text, Image).

---
## [2026-04-19] Phase 14: fepdf CLI Transformation

### 1. Phenomenon: Broken Test Samples after Crate Migration
- **Observation**: Shifting to a multi-crate workspace (SDK, Core, MCP) broke relative paths to PDF samples in nearly every test suite.
- **Cause**: Tests used `CARGO_MANIFEST_DIR` as the base, which changed from the project root to individual crate subdirectories.
- **Protocol Feedback**: Added **HDD Clause 7: Centralized Resource Path Management**. Use a designated helper in a test-utils crate (or high-level SDK) to resolve absolute workspace paths for test assets, rather than hardcoding relative `../` chains.

### 2. Phenomenon: Missing Font Data Fatal Error
- **Observation**: Documents using Standard 14 fonts without embedding caused the `Interpreter` to fail during rendering with "Missing font stream data".
- **Cause**: The parser correctly didn't find the font stream (as expected for Standard 14), but the renderer didn't have a metric-only fallback.
- **Protocol Feedback**: Updated **RR-15 Clause 16 (Context Propagation Guard)** to include "Requirement for Optional Non-Fatal Failures". High-level SDK components MUST distinguish between structural errors (invalid PDF) and rendering-quality gaps (missing fonts), allowing execution to continue for diagnostic and metadata tasks.

---

## [2026-04-19] Phase 16: Stream Filter & Security Hardening

### 1. Phenomenon: 'Corrupt Deflate Stream' on Encrypted PDFs
- **Observation**: Encryption was recognized, but content streams failed to decompress with "invalid block type" or "corrupt header".
- **Cause**: 
    - Attempted to decrypt `ObjStm` container, which corrupted the underlying objects.
    - Incorrect salt offset used for PBKDF2 in Revision 6 (U[32] vs U[40]).
    - Refusal to handle unaligned AES blocks (AESV3).
- **Protocol Feedback**: Added **HDD Clause 8: Compliance-First Security Auditing**. When a cryptographic layer is suspect, first verify decryption exclusions (ISO 7.6.2) and field offsets against the ISO 32000-2 bit-spec before attempting architectural fixes.

### 2. Phenomenon: Zlib Header Mismatch
- **Observation**: Some PDF producers use Raw Deflate (RFC 1951) instead of Zlib (RFC 1950) despite the `FlateDecode` filter name.
- **Cause**: Rigid adherence to Zlib headers in initial filter implementation.
- **Protocol Feedback**: Added **HDD Clause 9: Forgiving Decompression Fallback**. Decompression filters SHOULD attempt Raw Deflate fallback if the primary RFC-compliant header is missing or corrupted.
