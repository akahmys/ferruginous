# Ferruginous Rebirth Roadmap (v2.2)

The "Rebirth" project aims to establish the world's most robust and ISO-compliant PDF 2.0 toolkit, culminating in the **Ferruginous Flagship GUI Editor**.

---

## [COMPLETED] Phase 1 - 22: Foundation, Refinery & UA-2 Bridge
**Goal**: Build a high-purity PDF 2.0 data model and comprehensive diagnostic engine.

- [x] **Core Engine (M1-M62)**: Established Vello rendering, CJK hardening, and the Ferruginous Unified Architecture (FUA).
- [x] **The Refinery (M63-M65)**: Implemented high-speed, parallel ingestion with the Typed Arena and Handle system.
- [x] **UA-2 Engine (M66-M67)**: Finalized logical structure tree traversal and Matterhorn Protocol auditing.

## [COMPLETED] Phase 23: fepdf Deployment & Rendering Stabilization
**Goal**: Achieve absolute rendering parity and finalize the production-grade CLI toolkit.

- [x] **Rendering Stabilization**: Resolved complex graphics state leakage (EndPath `n` handling) and achieved parity for advanced technical manuals (Intel SDM baseline).
- [x] **High-Fidelity Color & Operator Hardening**: Implemented exhaustive operator dispatching (Rule 5) and high-fidelity color state preservation (Gray/RGB/CMYK/Lab).
- [x] **Type 3 Font Pipeline**: Normalized loading and metrics parsing for legacy CJK Type 3 fonts with `CharProcs` support.
- [x] **Active Refinement**: Implemented Unicode-native normalization and lossless color state serialization in the upgrade pipeline.
- [x] **Governance & Hardening**: Achieved 100% RR-15 compliance across the SDK and Render crates.

## Phase 24: High-Performance Interactive Canvas (The Glass Phase)
**Goal**: Deliver a 120fps fluid PDF viewing experience optimized for modern GPUs.

- [ ] **M68: Vello-Native egui Integration**:
    - [ ] **Direct Paint Callbacks**: Transition from headless texture blitting to direct egui-wgpu paint callbacks for zero-latency interaction.
    - [ ] **Retained Mode Rendering**: Implement scene-graph caching to minimize GPU redraws for static pages.
- [ ] **M68b: Viewport Virtualization**:
    - [ ] **Adaptive Tiling**: Implement tiled rendering for extreme zoom levels and large-format documents.
    - [ ] **Dynamic Memory Management**: Implement viewport-aware object sublimation and precipitation.
- [ ] **M68c: Selection & Interaction Engine**:
    - [ ] **Sub-pixel Hit Testing**: Precision text and path selection with real-time visual feedback.
    - [ ] **Semantic Highlighting**: Integrated rendering for annotations, links, and search results.

## Phase 25: The Semantic Forge (The Structure Phase)
**Goal**: Treat PDF as a logical structure tree first, and a visual representation second.

- [ ] **M69: Universal Structure Tree (UST)**:
    - [ ] **Interactive Tag Explorer**: A sidebar for visualizing and editing the logical structure (Tags/UA-2).
    - [ ] **Hierarchy Manipulation**: UI for semantic re-tagging and structure re-ordering.
- [ ] **M69b: Remediation Assistant**:
    - [ ] **Matterhorn Dashboard**: Real-time accessibility auditing with visual pointers to compliance errors.
    - [ ] **Alt-Text Studio**: A dedicated interface for managing alternative descriptions for complex figures.

## Phase 26: Professional Production & Security
**Goal**: Industrial-strength document integrity and output control.

- [x] **M70: The Forge Core**: Implemented PAdES-compliant signatures and Full Linearization with Hint Table generation. [COMPLETED]
- [ ] **M70b: Production Studio GUI**: Visual controls for Linearization, Vacuum, and Standard Compliance (PDF/A-4).
- [ ] **M70c: Atomic Redaction**: Secure UI for permanent visual and physical removal of sensitive stream data.

---

*Roadmap updated: 2026-05-11 (Phase 23 Stabilization Finalized)*
