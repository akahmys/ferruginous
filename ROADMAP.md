# Ferruginous Roadmap (ISO 32000-2:2020 Strict)

## 1. Constitution

- **[RR-15](.agent/protocols/RELIABLE_RUST_15.md)**: Mechanically apply 15 safety constraints.
- **[HDD](.agent/protocols/HDD_PROTOCOL.md)**: Adhere strictly to specification-first, automated verification harness-driven development.
- **Target**: ISO 32000-2:2020 (PDF 2.0) Only
- **MSRV**: 1.94 / Edition 2024

## 2. Parallel Principles

- **Track-Driven**: Define SDK (Track S) and App (Track A) as parallel tracks, accelerating development while keeping them linked.
- **SSoT Sync**: Artifacts from each track must always be synchronized with the design specifications under `specs/` and `ROADMAP.md`.

## 3. AI Implementation Phases

### Phase 1: Core Foundation [Complete]

- **M1-M4: Establishing Physical Structure**: COS parsing, DrawOp, DisplayList.
- **M5-M8: Logical Structure & Resolvers**: Page Tree, Resource Dictionaries, Indirect Object Resolution.

### Phase 2: Content Parsing [Complete]

- **M9-M14: Filters & Graphics Instructions**: FlateDecode, ASCIIHexDecode, Path Painting.

### Phase 3: SDK Refinement [Complete]

- **M15-M18: Resource Management & Consistency Validation**: Images, Font Infrastructure, Arlington Validation.

### Phase 4: Display & UX [Complete]

- **M23-26: Vello Rendering Integration**: Incorporating WGPU/Vello pipeline into Eframe.

### Phase 5: Interactive Foundations [Complete]

- **M30-A: Copy & Unicode Extraction** [Complete]
  - Unicode restoration for selection areas and clipboard integration (Clause 9.10).
- **M30-S: Realizing Incremental Save** [Complete]
  - Persistence of changes through integration of physical serializers (Clause 7.5.6).
- **M31-A: Annotation UI** [Complete]
  - Visual editing of highlights, rectangles, arrows, and freehand (Clause 12.5).
- **M32-A: Full-Text Search & Outline** [Complete]
  - In-document search and sidebar table of contents (Clause 12.3.3).

### Phase 6: Forms & Layers [Complete]

- **M34-S: AcroForms (Interactive Forms)**
  - Parsing form fields and generating appearance streams (Clause 12.7).
- **M35-A: Form Input & Layer Controls (UI Integration)**
  - Saving inputs on the UI and switching OCG layers (Clause 14.11).

### Phase 7: Advanced Graphics & Compliance [Complete]

- **M36-S: ICC Profiles & Color Management** [Complete]
  - Integration of color management systems (e.g., lcms2) (Clause 8.6).
- **M37-S: Transparency Groups & Advanced Shading** [Complete]
  - Transparency groups, blend modes, and Type 4-7 shading (Clauses 8.7, 11).

### Phase 8: Security & Trust [Complete]

- **M38-S: AES-256 (Rev 6) Encryption** [Complete]
  - Compliant advanced password protection and decryption (Clause 7.6.4).
- **M39-S: Digital Signatures (PAdES/LTV)** [Complete]
  - Verification and application of signatures using public key infrastructure (Clause 12.8).
- **M40-A: Secure Redaction** [Complete]
  - Physical and irreversible deletion of content (Clause 12.5.6.23).

### Phase 9: Accessibility & Structure [Complete]

- **M41-S: Logical Structure & Tagged PDF** [Complete]
  - RoleMap/ClassMap resolution and document structure tree parsing (Clause 14.7).
- **M42-A: Structure Display & Tag Validation UI** [Complete]
  - Sidebar display of structure tree and Tagged PDF consistency validation (Clause 14.8).

### Phase 10: Finalization & Release [Complete]

- **M43-S: 3D & Multimedia Analysis** [Complete]
  - Parsing RichMedia and 3D annotations and verifying specifications (Clause 13).
- **M44-S: Arlington Full Compliance Suite** [Complete]
  - Quality assurance via automated recursive schema validation of all objects.
- **M45: Optimization & Final Integration** [Complete]
  - Completion of parallel rendering optimization via Rayon and UI integration.

### Phase 11: Governance & Quality [Complete]

- **M46-G: Restructuring Governance Protocols**: Separating roles and deduplicating `GEMINI.md` (Charter) and `PLANNING_PROTOCOL.md` (Management). [Complete]
- **M47-Q: Strengthening RR-15 Compliance**: Preparing `verify_compliance.sh` and achieving zero warnings/violations across the SDK and UI. [Complete]
- **M48-A: Modernizing the UI Layer**: logic flattening using guard clauses and strengthening invariants by eliminating `unwrap()`. [Complete]

### Phase 12: Professional Orchestration [Complete]

- **M49-A: Page Orchestrator (UI)**: UI for visual page reordering, extraction, and merging using thumbnail grids. [Complete]
- **M49-S: Document Manipulation API (SDK)**: Safe Page Tree operations that prevent resource inconsistency and page importing from other documents. [Complete]

### Phase 13: Engineering Precision [Complete]

- **M50-S: Geometric Proximity Search (SDK)**: Precision snap point calculation engine for vector paths. [Complete]
- **M50-A: Snap UX & Precision Measurement (UI)**: CAD-grade snapping for visual measurement of distance and area and scale management. [Complete]

### Phase 14: Semantic Intelligence [Complete]

- **M51-S: Automated Tag Structuring & Pattern Detection (SDK)**: AI-driven automatic generation of logical structure (Tagged PDF) and pattern extraction for sensitive information. [Complete]
- **M51-A: Tag Tree Editor (UI)**: Visual editor to "repair" logical structures via mouse operations and redaction assistant. [Complete]

### Phase 15: Creative Mastery [Complete]

- **M52-A: Modeless Workflow (UI)**: Modeless context menu UX driven by selected objects, eliminating all fixed menus. [Complete]
- **M52-S: Reflow Editing & 3D Rendering (SDK)**: Paragraph-based text reflow editing and actual rendering of U3D/PRC models via WGPU. [Complete]

### Phase 16: Architectural Resilience [Complete]

- **M53-S: Abstraction of Rendering Backend (Render Bridge)** [x]
  - Eliminate direct dependency on Vello and build an abstraction layer using the `RenderBackend` trait.
- **M54-G: Establishing Platform Independence** [x]
  - Define interfaces considering future bridge connections to WASM / Web Canvas and organize dependency structure for `ferruginous-ui`.

### Phase 16.5: Quality & Feature Refinement [Complete]

- **M55-Q: Resolving Technical Debt (Sprint)** [x]
  - Enforcing `missing_docs`, `redundant_clone`, and safe type casting across all SDK/UI layers.
- **M56-G: Improving Documentation Quality** [x]
  - Adding commentary based on ISO 32000-2 clause citations to major APIs.

### Phase 18: Multibyte Text Rendering Precision [Complete]

> [!NOTE]
> **All Steps Complete (2026-04-14)**:
> Implementation and verification of CIDToGIDMap resolution, vertical origin shifts, and glyph rotation based on WMode are complete.
> Passed final quality assurance via `diag_layout` and `verify_compliance.sh`.

- **M59-S: FontMatrix Standardization & Composite Font Sync (SDK)** [x]
  - Unifying default FontMatrix (0.001) in compliance with ISO 32000-2 and implementing matrix inheritance from child CIDFont elements in Type 0 fonts.
- **M60-S: Strict Glyph Width Synchronization (SDK)** [x]
  - Forcing synchronization between PDF `W` (Widths) arrays and font file glyph metrics to physically eliminate overlapping (bunching) of Japanese characters.
- **M61-A: Full Renderer Coordinate Synchronization (UI)** [x]
  - Fully integrating the cumulative transformation matrix (CTM) into text rendering, eliminating 100% of misalignment with shapes and borders.

## 4. Process Rules

- **Parallel Sync**: Manage synchronization points for Track S and Track A in `task.md`.
- **Atomic Sync**: Synchronize `task.md` and `ROADMAP.md` before every report.

## 5. Next Generation Roadmap: Ferruginous "Sentinel" v2.0

Based on the completion of Phase 18, we define next-generation phases that balance "Robustness" with "Visual Experience."

### Phase 19: High-End Compliance & Archiving [x]

Achieving full mastery of professional standards.

- **M62-S: PDF/A-4 & PDF/X-6 Strict Compliance (Strict Write)**
    - M62.1-S: Overhaul of XMP Metadata Engine (ISO 16684-1 compliant) [x]
    - M62.2-S: Implementation of Unicode Integrity Checker and Auto-repair Layer [x]
    - M62.3-S: Bidirectional management of Associated Files (PDF/A-4f support) [x]
- **M63-S: High-Precision Color Management (Spot Color & ICC v4)**
    - M63.1-S: Page-level OutputIntents analysis and rendering reflection [ ]
    - M63.2-S: LUT generation engine for Black Point Compensation (BPC) [ ]
    - M63.3-S: Parsing of spectral data (CxF/X-4) and spot color simulation [ ]

### Phase 20: Visual Excellence & Design System ("Sentinel UI") [ ]

Building an original design system "Sentinel UI" based on Egui.

- **M64-A: Premium UI Theme Construction**
    - M64.1-A: Creation of the `Sentinel-Theme` crate (HSL Tokens & Dark/Light switch) [ ]
    - M64.2-A: Implementation of frameless windows and custom title bars (macOS/Windows) [ ]
    - M64.3-A: Integration of built-in product fonts (Inter / Outfit) and dynamic heapification [ ]
- **M65-A: Motion & Micro-interactions**
    - M65.1-A: Integration of motion curve library (Spline-based easing) [ ]
    - M65.2-A: Physics-based animation for panel transitions and modal overlays [ ]

### Phase 21: Performance & Universal Platform [ ]

- **M66-S: Linearization (Fast Web View) Support**
  - M66.1-S: Hint table generation and optimizer for object order optimization [ ]
  - M66.2-S: Incremental loading protocol for large files [ ]
- **M67-P: WGPU-WASM Performance Sprint**
- **M68-A: Initial implementation of Reflow Viewing Mode (Liquid Mode)**

### Phase 22: Advanced Interactive Core [ ]
- **M69-S: Safe AcroJS Sandbox (RR-15 compliant)**
  - Implementing a compliant JS execution environment in a secure sandbox.
- **M70-A: Refreshing the Interactive Form UI**
  - Real-time validation and a seamless input experience.

### Phase 23: Data PDF & Hybrid Containers [ ]
- **M71-S: Bidirectional Management of Associated Files**
  - Implementing Clause 14.13 for utilizing PDF as a data container.

### Phase 24: AI-Native Document Intelligence (Deferred) [ ]
- **M72-S: Semantic Extraction & LLM Integration**
- **M73-A: Interactive AI Assistant UI**
