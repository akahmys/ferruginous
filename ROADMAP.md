# Ferruginous Rebirth Roadmap (v2.1)

The "Rebirth" project aims to achieve the world's most robust and ISO-compliant PDF 2.0 toolkit, culminating in the **Ferruginous Flagship GUI Editor**.

---

## [COMPLETED] Phase 1 - 22: Foundation, Refinery & UA-2 Bridge
**Goal**: Build the world's highest purity PDF 2.0 data model and diagnostic engine.

- [x] **Core Engine (M1-M62)**: Established Vello rendering, CJK hardening, and the Ferruginous Unified Architecture (FUA).
- [x] **The Refinery (M63-M65)**: High-speed, parallel ingestion pipeline with Typed Arena and Handle system.
- [x] **UA-2 Engine (M66-M67)**: Full logical structure tree traversal and Matterhorn Protocol auditing.

## [COMPLETED] Phase 23: fepdf Deployment & Rendering Hardening
**Goal**: Achieve ISO-compliant rendering stability and finalize CLI utilities.

- [x] **Rendering Stability**: Robust coordinate mapping, CMap v2 engine, and strict ICC profiling for Japanese PDFs.
- [x] **Active Refinement**: Unicode-native normalization and unified font mapping during ingestion.
- [x] **Governance & Security**: 100% compliance with Rule 1 (RR-15) and integrated PII/Secret scanning.

## Phase 24: High-Performance Interactive Canvas (The Glass Phase)
**Goal**: Build a butter-smooth PDF viewing experience that feels "alive" on modern hardware.

- [ ] **M68: Vello-Native egui Integration**:
    - [ ] **Direct Paint Callbacks**: Move from headless texture blitting to direct egui-wgpu paint callbacks for 120fps interaction.
    - [ ] **Retained Mode Rendering**: Implement scene caching to minimize GPU redraws for static pages.
- [ ] **M68b: Infinite Scroll & Virtualization**:
    - [ ] **Adaptive Tiling**: Tiled rendering for large documents and high zoom levels.
    - [ ] **Viewport Management**: Viewport-aware page loading and unloading for low memory footprint.
- [ ] **M68c: Selection Engine**:
    - [ ] **Sub-pixel Precision**: Accurate text selection with visual feedback and hit testing.
    - [ ] **Semantic Highlighting**: Highlighting links, annotations, and search results.

## Phase 25: The Semantic Forge (The Structure Phase)
**Goal**: Treatment of PDF as a logical structure tree first, visual representation second.

- [ ] **M69: Universal Structure Tree (UST)**:
    - [ ] **Visual Tree Side Panel**: Traversable and editable view of the logical structure (Tags).
    - [ ] **Direct Hierarchy Manipulation**: UI for re-ordering and re-tagging (e.g., Paragraph -> Heading).
- [ ] **M69b: Remediation Assistant**:
    - [ ] **Matterhorn Dashboard**: Real-time accessibility audit (UA-2) with visual pointers to errors.
    - [ ] **Alt-Text Studio**: dedicated interface for managing alternative descriptions for complex figures.
- [ ] **M69c: Reflow & Accessibility Views**:
    - [ ] **Reflow Mode**: Extracting and re-rendering text for responsive reading.
    - [ ] **Screen Reader Sync**: Ensuring the GUI focus follows the logical structure tree.

## Phase 26: Professional Production & Security (The Finality Phase)
**Goal**: Industrial-strength output and document integrity.

- [x] **M70: The Forge Core**: PAdES-compliant signatures, **Full Linearization** with Hint Table generation, and optimized save engine. [COMPLETED]
- [ ] **M70b: Save Studio GUI**: Visual controls for Linearization, Vacuum, and Standard Compliance (PDF/UA-2, A-4).
- [ ] **M70c: Digital Signature Studio**: Visual signature placement and LTV (Long-Term Validation) management.
- [ ] **M70d: Atomic Redaction**: Secure visual redaction that physically destroys stream data.

## Phase 27: Workspace & Ecosystem
**Goal**: Making Ferruginous the center of the professional PDF workflow.

- [ ] **M71: Multi-Document Workspace**: Tabbed interface and cross-document page drag-and-drop.
- [ ] **M72: WASM Plugin API**: Allowing third-party extensions for custom auditing and transformations.

---

## 5. Progress History (Highlights)

### [x] Phase 23 (First Half): Rendering Hardening
- [x] **M67**: Achieved stable rendering of complex Japanese PDFs. Implemented **Advanced Metadata Sublimation** (Info <-> XMP), **Advanced Vacuum** (Contiguous ID Remapping), and **Primary Linearization** (Object Reordering) in the export engine.

### [x] Phase 22: fepdf & UA-2 Bridge (2.2)
- [x] **M66/M67**: Restored core engine, implemented iterative `StructureVisitor`, and unified `MatterhornAuditor`. Achieved 100% build health with new Arena handle system.

### [x] Phase 21: The Refinery Evolution (Refinery 2.1)
- [x] **M64: Concurrent Pipeline**: Integrated `rayon`, `chardetng`, and `moxcms` for parallel high-purity ingestion.
- [x] **M63/M65: Ingestion & Handle System**: Transitioned to `lopdf` physical parsing and handle-based memory model.

... (Previous Phases 1 - 20 summarized)

---
*Roadmap updated: 2026-04-28 (Phase 24-27 GUI Expansion)*
