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

## Phase 12: Advanced Web Optimization [COMPLETED]

- [x] **M32-LIN: Linearization Dictionary & Object Reordering**
- [x] **M33-LIN: Multi-pass Serialization Engine**
- [x] **M34-LIN: Primary Hint Stream Generation (Page Offset & Shared Objects)**
- [x] **M35-LIN: Upgrader Integration (`--linearize` flag)**
- [x] **Phase Closure (Ritual)**

---
*Roadmap updated: 2026-04-19 (Phase 12 Planning Initiated)*
