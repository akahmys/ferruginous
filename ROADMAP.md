# Ferruginous Rebirth Roadmap (v2.0)

The "Rebirth" project aims to achieve the world's most robust and ISO-compliant PDF 2.0 toolkit.

## Phase 1: Core Foundation [Complete]

- **M1-CORE: Zero-copy Lexer** [x]
- **M2-CORE: Recursive Descent Parser** [x]
- **M3-CORE: RR-15 Type System** [x]

## Phase 2: Document Structure [Complete]

- **M4-DOC: XRef Table & Stream Parsing** [x]
- **M5-DOC: Object Resolution (Lazy Loading)** [x]
- **M6-DOC: Non-recursive Page Tree Walk** [x]

## Phase 3: Project Architecture & Quality [Complete]

- **M7-ARCH: Protocol Standardization** [x]
- **M8-ARCH: Documentation Synchronization** [x]
- **M9-CORE: RR-15 Hardening & Refactoring** [x]

### [x] Phase 6: Functional Verification & Crash Stabilization
- [x] Comprehensive test suite execution (日・英 10 files)
- [x] Implementation of full Object-based Interpreter (Fixed TJ parsing)
- [x] Support for Resource Inheritance & Reference Resolution
- [x] GPU Rendering stabilization (Indexed color detection & Buffer guards)
- [x] 100% SUCCESS rate for core rendering/text extraction

## Phase 4: Graphics & Rendering (`ferruginous-render`) [Complete]

- **M10-RENDER: Vello Integration** [x]
- **M11-RENDER: CTM (Cumulative Transformation Matrix) Engine** [x]
- **M12-RENDER: Primitive Drawing (Path, Shape, Dash, Clip)** [x]

## Phase 5: High-Precision Typography [Complete]

- **M13-FONT: CIDFont & CMap Resolution** [x]
- **M14-FONT: Glyph Positioning & Width Synchronization (Skrifa)** [x]

## Phase 6: Document MCP & Hardening (`ferruginous-mcp`)

- **M15-MCP: Protocol Server Implementation** [x]
- **M16-MCP: Structural Compliance Auditing Tool** [x]
- **M17-HARD: Architectural Hardening (XRef Stream, Caching, Parser Lookahead)** [x]

## Phase 7: Advanced Content & Transparency [x]

- **M17-CONTENT: Advanced Text Interpreter (Stateful positioning & Showing)** [x]
- **M18-CONTENT: Image Streams (DCTDecode & multiple filters)** [x]
- **M19-CONTENT: Form XObjects & Resource Nesting (16-level limit)** [x]
- [x] Phase 7: Transparency & Hardening
  - [x] Implement BlendModes and Alpha support (ca, CA, BM)
  - [x] Enforce Context Propagation Guard (RR-15 Clause 16)
  - [x] Audit-ready codebase with zero Clippy warnings
- [x] **Phase 7 Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (RR-15 Audit Passed)
  - [x] Rule Refinement & Retrospective ([docs/retrospectives/phase_7.md])
  - [x] Update Documentation (ROADMAP, README)
  - [x] GitHub Synchronization (Commit/Push Ready)

## Phase 8: Text & Fonts [Complete]

- [x] M20-FONT: Complete CIDFont and CMap resolution
- [x] M21-FONT: Standard 14 font fallbacks
- [x] M22-CONTENT: Advanced text rendering pipeline
- [x] **Phase 8 Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (RR-15 Audit Passed)
  - [x] Rule Refinement & Retrospective ([docs/retrospectives/phase_8.md])
  - [x] Update Documentation (ROADMAP, README)

## Phase 9: Security & Protection

- **M23-SEC: PDF Encryption (AES-256, Unicode Passwords)** [x]
- **M24-SEC: Digital Signatures & LTV (PAdES)** [x]
- [x] **Phase Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (Data Structure & Data Flow Audit)
  - [x] Rule Refinement & Retrospective (`REFLECTION_PROTOCOL`)
  - [x] Update Documentation (ROADMAP, README, DESIGN_SPEC)
  - [x] GitHub Synchronization (Commit/Push)

## Phase 10: Standards & Compliance [Complete]

- **M25-COMP: PDF/A-4 & PDF/X-6 Conformance** [x]
- **M26-COMP: Tagged PDF & Accessibility (PDF/UA-2)** [x]
- **M27-COMP: Associated Files (PDF/A-4f)** [x]
- [x] **Phase Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (Data Structure & Data Flow Audit)
  - [x] Rule Refinement & Retrospective (`REFLECTION_PROTOCOL`)
  - [x] Update Documentation (ROADMAP, README, DESIGN_SPEC)
  - [x] GitHub Synchronization (Commit/Push)

## Phase 11: Production & Ecosystem [Complete]

- **M28-SDK: High-Performance CLI Tooling** [x]
- **M30-SDK: WASM / Web Integration** [x]
- **M31-SDK: v1.0 Production Readiness & Audit** [x]
- [x] **Phase Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (Data Structure & Data Flow Audit)
  - [x] Rule Refinement & Retrospective (`REFLECTION_PROTOCOL`)
  - [x] Update Documentation (ROADMAP, README, DESIGN_SPEC)
  - [x] GitHub Synchronization (Commit/Push)

## Phase 12: Advanced Web Optimization [Complete]

- [x] **M32-LIN: Linearization Dictionary & Object Reordering**
- [x] **M33-LIN: Multi-pass Serialization Engine**
- [x] **M34-LIN: Primary Hint Stream Generation (Page Offset & Shared Objects)**
- [x] **M35-LIN: Upgrader Integration (`--linearize` flag)**
- [x] **Phase Closure (Ritual)**

## [COMPLETED] Phase 13: Ferruginous Modern Standard Suite (Recovery & Hardening)
- [x] Integrate Info/Audit CLI targets into `fepdf`.
- [x] Implement UTF-16BE decoding for broad metadata support.
- [x] Implement structural tree visualizer (`--structure`).
- [x] Implement object garbage collection (`--vacuum`) and metadata stripping.
- [x] **Restored Document engine integrity (Resolved all 23+ compilation errors).**
- [x] Final end-to-end verification of the auditing engine.

## [COMPLETED] Phase 14: Ferruginous fepdf CLI Transformation
- [x] **M40: CLI Transformation**: Developed `fepdf` as the official subcommand-based CLI sub-project.
- [x] **M41: Feature Enrichment**:
  - [x] Implemented Object Cloning engine with recursive ID remapping in SDK.
  - [x] Integrated `merge`, `split`, and `rotate` functional handlers.
- [x] **M44: Scavenger Mode**: Implemented robust object marker scanning in the core engine.
- [x] **M42: Multi-platform Distribution**: Created `Makefile` for distribution of fepdf binaries across Mac/Win/Linux.

- [x] **Phase 14 Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (Resolved trait regressions and sample path issues)
  - [x] Rule Refinement & Retrospective ([docs/retrospectives/phase_14.md])
  - [x] Update Documentation (ROADMAP, README)


## [COMPLETED] Phase 15: CJK Font Hardening
- [x] **M45: Robust ToUnicode Decoding**: Implemented mandatory stream decoding (De-compression) for mapping dictionaries.
- [x] **M46: Character Mapping Hardening**: Developed bidirectional padding normalization for subsetted CMap entries.
- [x] **M47: Legacy Encoding Support**: Integrated `encoding_rs` for robust Shift-JIS/CP932 fallback.
- [x] **Verified zero-mojibake rendering** of complex Japanese legal documents (e.g., Constitution of Japan).

- [x] **Phase 15 Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (100% Clippy & Test success)
  - [x] Rule Refinement & Retrospective ([docs/retrospectives/cjk_mojibake_resolution.md])
  
## [COMPLETED] Phase 16: Stream Filter & Security Hardening
- [x] **M48: Filter Hardening**: Implemented Raw Deflate (RFC 1951) fallback and improved skip logic for non-standard EOL markers.
- [x] **M49: Advanced Predictors**: Integrated TIFF Predictor 2 (Horizontal Differencing) and variable bit-depth support.
- [x] **M50: Security Integration**:
  - [x] Implemented legacy key derivation (Algorithm 3.2, Revision 2/3/4).
  - [x] Hardened AESV3 (Revision 6) decryption via specialized partial block masking.
  - [x] Enforced ISO 32000-2 compliance: Bypassed decryption for `ObjStm` and `XRef` streams.
- [x] **Verified stabilization** of complex administrative and legal Japanese PDF samples (cao_guide.pdf, mext_report.pdf).

- [x] **Phase 16 Closure (Ritual)**
  - [x] Codebase Validation & Refactoring (Resolved AESV3 salt offset and trailer resolution bugs)
  - [x] Rule Refinement & Reflection ([.agent/hardening_rules.md], [.agent/reflections.md])
  - [x] Update Documentation (ROADMAP, README)

## Phase 17: Arlington Predicates & Structural Validation
- [x] **M51: Predicate Engine**: Implement `nom`-based parser for ISO 32000-2 Arlington Model predicates.
- [x] **M52: Evaluation Logic**: Develop a deterministic evaluator for complex validation conditions (SinceVersion, Required, etc.).

## [COMPLETED] Phase 19: Vertical Rendering & PDF 2.0 Standardization
- [x] **M56: Vertical Writing (WMode 1)**: Implemented coordinate transformation logic for descending text advance in CJK vertical layout.
- [x] **M57: PDF 2.0 Upgrade Engine**: Enhanced the SDK writer to support targeted version upgrades (`save_as_version`) including mandatory metadata injection.
- [x] **M58: UI Interaction Hardening**: Integrated `Command + ScrollWheel` zoom and smooth panning into the egui-Vello bridge.
- [x] **Verified vertical flow compliance** with Japanese literary samples (bokutokitan.pdf) converted to ISO 32000-2.

---
*Roadmap updated: 2026-04-20 (Phase 19 Completed)*
