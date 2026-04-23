# Ferruginous Rebirth Roadmap (v2.1)

The "Rebirth" project aims to achieve the world's most robust and ISO-compliant PDF 2.0 toolkit, culminating in the **Ferruginous Flagship GUI Editor**.

---

## [COMPLETED] Phase 1 - 20: Foundation & Unified Architecture
- [x] **M1 - M31**: Core engine development, Vello rendering, CJK font hardening, and security integration.
- [x] **M60-M62: Ferruginous Unified Architecture (FUA)**: Consolidated `core` engine and established the SDK/App/CLI hierarchy.

## [COMPLETED] Phase 21: The Refinery Evolution
**Goal**: Establish position as a "Strict PDF 2.0 Translator" and build the highest purity data model.

- [x] **M63: Ingestion Gateway**: Adopt `lopdf` as the physical parsing gateway and implement high-speed ownership transfer to `PdfArena`. [COMPLETED]
- [x] **M64: Concurrent Refinery Pipeline**: Parallelized text/color/metadata normalization via `rayon`. [COMPLETED]
- [x] **M65: Typed Arena & Handle System**: Maximize memory efficiency and safety. [COMPLETED]

## [COMPLETED] Phase 22: fepdf & UA-2 Bridge (CLI Hardening)
**Goal**: Transform `fepdf` into a world-class diagnostic and accessibility tool.

- [x] **M66: UA-2 Structure Engine**:
    - [x] **Structure Tree**: Implement full traversal and validation of the logical structure tree (Clause 14.7). [COMPLETED]
    - [x] **Matterhorn Protocol**: Detailed auditor based on the failure conditions for UA-2. [COMPLETED]
    - [x] **Alt-Text Audit**: Diagnostic logic for non-textual content descriptions. [COMPLETED]
- [x] **M67: fepdf Readiness (M67 Prep)**:
    - [x] **Core Restoration**: Stabilized RefCell-based Arena and modernized SDK interpreter. [COMPLETED]
    - [x] **Font & CMap Engine**: Functional Unicode mapping for structural auditing. [COMPLETED]
    - [x] **Build Integrity**: 100% clean check across all crates. [COMPLETED]

## [IN PROGRESS] Phase 23: fepdf Field Deployment & Rendering Hardening
**Goal**: Finalize command-line utilities and achieve ISO-compliant CJK rendering stability.

- [x] **M67: Rendering Engine Hardening**:
    - [x] **Coordinate Mapping**: Implemented robust non-zero MediaBox origin handling and coordinate space inversion. [COMPLETED]
    - [x] **CMap Engine v2**: Smart byte-length detection and support for complex `beginbfrange` mappings. [COMPLETED]
    - [x] **Text Matrix Advancement**: Implemented strict Tm/TLM synchronization for multi-line layout. [COMPLETED]
- [x] **M67d: Unicode-Native Arena & Color Hardening**:
    - [x] **Restructuring Ingest**: Implement 2-pass "能動的精製" (Active Refinement) to normalize content streams to UTF-8. [COMPLETED]
    - [x] **Unified Font Mapping**: Construct authoritative Unicode-to-GID maps during ingest. [COMPLETED]
    - [x] **Strict ICC Profiling**: Implement full `ICCBased` color space support using `moxcms`. [COMPLETED]
    - [x] **Operator Coverage**: Complete implementation of `g`, `G`, `k`, `K`, `Do` (Form), and Marked Content. [COMPLETED]
- [ ] **M67e: fepdf Release Features (Rich Reporting & Refinery Controls)**:
    - [ ] **Ingestion Options**: Expose `active_refinement`, `metadata_sublimation`, and `color_policy` controls.
    - [ ] **Rich Reporting**: Exportable JSON/Markdown compliance reports, including **Embedded Font Audit**.
    - [ ] **Metadata Recovery**: Implement full Document Info to XMP sublimation.
- [x] **M67f: Antigravity Standard Ruleset**: Reorganized all project protocols and agent rules into the standardized `.antigravity/` tiered structure and unified project language to English. [COMPLETED]

## Phase 24: Ferruginous Flagship GUI (Application)
**Goal**: Provide the world's best PDF 2.0 editing experience using the hardened UA-2 library.

- [ ] **M68: High-Fidelity Workspace**: Vello-accelerated rendering and asynchronous status visualization.
- [ ] **M69: Semantic Structure Editor**: Direct manipulation of the logical structure tree (Tags).
- [ ] **M70: The Forge (Export & Security)**: PAdES-compliant signatures and optimized PDF 2.0 save engine.

---

## 5. Progress History (Highlights)

### [x] Phase 23 (First Half): Rendering Hardening
- [x] **M67**: Achieved stable rendering of complex Japanese PDFs (nihonkokukenpou.pdf, bokutokitan.pdf) by hardening the coordinate mapping and CMap parsing logic.

### [x] Phase 22: fepdf & UA-2 Bridge (2.2)
- [x] **M66/M67**: Restored core engine, implemented iterative `StructureVisitor`, and unified `MatterhornAuditor`. Achieved 100% build health with new Arena handle system.

### [x] Phase 21: The Refinery Evolution (Refinery 2.1)
- [x] **M64: Concurrent Pipeline**: Integrated `rayon`, `chardetng`, and `moxcms` for parallel high-purity ingestion.
- [x] **M63/M65: Ingestion & Handle System**: Transitioned to `lopdf` physical parsing and handle-based memory model.

... (Previous Phases 1 - 20 summarized)

---
*Roadmap updated: 2026-04-24 (Phase 23 Rule Reorganization Completed | Rendering Hardening Verified)*
