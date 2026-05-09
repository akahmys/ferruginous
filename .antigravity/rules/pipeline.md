# The Ferruginous Processing Pipeline

This document defines the canonical four-phase pipeline for transforming a physical PDF file into a compliant, rendered, and semantically understood document.

## 1. Overview: Normalization-at-Load
Ferruginous adheres to the principle of **"Normalization-at-Load."** The goal is to resolve all ambiguities, inheritance, and non-standard data structures as early as possible, ensuring that the downstream stages (Interpretation and Rendering) are purely deterministic and side-effect-free.

---

## 2. The Four Phases

### Phase 1: Ingestion & Decryption (Physical Layer)
- **Responsibility**: Resolve physical addressing and access control.
- **Actions**:
    - **XRef Resolution & Inhalation**:
        - Map physical `ObjectId` pairs to logical `Handle<Object>` pointers. This decouples the logical structure from physical file offsets.
        - Heuristically repair broken XRef tables if found during inhalation.
    - **Pass 0 Decryption**:
        - **Iterative Traversal**: Decrypt all strings and streams using a stack-based (non-recursive) walk to prevent stack overflow in deeply nested documents.
        - **Fidelity Cleanup**: Explicitly remove the `/Encrypt` entry from the trailer after decryption to satisfy strict Adobe/ISO requirements (avoiding Error 135).
- **State**: Objects are loaded into `PdfArena` as a flat, addressable set of handles.

### Phase 2: Normalization & Pre-Analysis (Logical Layer)
- **Responsibility**: Transform "Dirty" data into "Ideal" PDF 2.0 structures.
- **Actions**:
    - **Decompression**: Temporarily decompress streams for inspection.
    - **Resource Inheritance Flattening**:
        - Traverse the `Parent` chain of the Page Tree to collect and flatten `Resources` (Fonts, XObjects, ColorSpaces).
        - Every content stream is assigned a complete, self-contained resource map, eliminating the need for context-sensitive lookups during rendering.
    - **Content Stream Sublimation (Body Normalization)**:
        - **Operator Atomicity**: Complex operators with implicit side effects (e.g., `TD`, `"`, `'`) are expanded into a sequence of atomic IR commands (e.g., `SetTextLeading`, `MoveToNextLine`, `ShowText`).
        - **Writing Mode Injection**: Explicitly inject `SetWritingMode` commands into the IR stream whenever a font is selected (`Tf`). This flattens the Writing Mode state and ensures deterministic layout for mixed horizontal/vertical streams.
        - **Corruption Resurrection**: Detect and repair content streams that contain non-standard or "leaked" debug data (e.g., Rust debug output).
    - **Font Reconstruction & Hardening (Structural Normalization)**:
        - **Global Font Registry**: Collect all discovered fonts document-wide. If a local resource chain is broken, fallback to this global registry.
        - **Robust Extraction**: Fallback search in parent dictionaries if `FontDescriptor` file entries are missing (ISO 32000 Hardening).
        - **Format Identification**: Subtype identification relies strictly on binary signatures (Magic Bytes: `OTTO`, `\0\1\0\0`, `ttcf`, `true`, `%!`).
        - **Virtual OpenType (SFNT) Wrapping**: Wrap CFF/Type 2 outlines in SFNT containers; prepare Type 1 for transcoding via segment length tracking.
        - **Metric & Mapping Injection**: Inject PDF-authoritative `hmtx` and `cmap` tables into reconstructed SFNT buffers.
        - **Type 0 & CIDFont Robustness**: 
            - Ensure logical `Type 0` fonts correctly resolve their descendant `CIDFont` for metrics.
            - **Metrics Parsing Branching**: Explicitly distinguish between `/Widths` (Standard fonts) and `/W` (CIDFonts). Standalone `CIDFontType0/2` resources MUST use `/W` parsing to avoid defaulting to 1000 units, which causes layout regressions (wide spacing).
    - **Handle Stability & Lifecycle Invariance (RR-15 Hardening)**:
        - **Object-Centric Modeling**: All high-level structural components (Catalog, Page, StructTreeRoot) MUST hold stable `Handle<Object>` references. Direct storage of volatile `DictHandle` is prohibited in persistent models.
        - **Late-bound Dictionary Resolution**: Attribute access must rely on "Late-binding" (resolving `Handle<Object>` to `DictHandle` at the time of access). This ensures that even if a `ParallelRefinery` pass regenerates the underlying dictionary, the model's reference remains valid and points to the latest state.
        - **Normalization Synergy**: While normalization "bakes" inherited attributes into local dictionaries for performance, late-binding ensures this "clean baseline" is accessed safely regardless of subsequent memory-level reallocations.
    - **Semantic Mapping & Character Resolution Chain**:
            - **Authoritative CID Resolution (Phase 2.5 Logic)**:
                - For CID-keyed (Composite) fonts, the primary resolution path is: **`Character Code` → `CMap` → `CID` → `GID`**.
                - **Lying Identity Exception (Western)**: Detect "Lying Identity" fonts (`Ordering: Identity` AND Non-CJK subset). For these fonts, the internal `cmap` table is treated as unreliable.
                - **Priority -1 (Normalization Truth)**: During Virtual OpenType reconstruction for Lying Identity fonts, prioritize **Identity Mapping (CID == GID)**. Inject this truth into the synthesized SFNT's `cmap` table to preemptively resolve Mojibake before the rendering phase.
                - **Authoritative CJK Path**: For CJK CID-keyed fonts (detected via Registry/Ordering or heuristics), the **`CMap` → `CID`** path is absolute. `CID -> GID` resolution via `CIDToGIDMap` (or Identity) is strictly prioritized over Unicode hints to preserve document-specific glyph selection (e.g., variant forms, IVS).
        - **Multi-Pass Normalization Strategy**:
            - **Priority 1 (Physical Truth)**: Extract mappings from the embedded font file's physical `cmap` tables.
            - **Priority 2 (Document Truth)**: Integrate `ToUnicode` mappings, but only allow overrides if they yield a non-zero (valid) GID.
            - **Priority 3 (System Fallback)**: Populate missing entries from system fonts to ensure renderability.
            - **Priority 4 (Heuristics)**: Use Adobe-Japan1 (AJ1) or custom encoding heuristics for legacy documents.
        - **Zero-Guesswork GID Tables**: All mappings are flattened into an authoritative `unicode_to_gid` and `cid_to_gid` cache during loading.
        - **Character Resolution & Suppression Rules**:
            - **PUA Hint Preservation (CID-Keyed Fonts)**: Do NOT suppress glyph resolution hints mapped to the Private Use Area (PUA, e.g., `0xF0000 + CID`) if the font is CID-keyed. In Japanese and other CJK documents, these "hints" are often valid internal identifiers for CIDs that lack an authoritative `ToUnicode` map.
            - **Artifact Suppression**: Artifact suppression (e.g., control characters, specific symbols like `⓪`) is reserved for simple (non-CID) fonts where heuristics are more prone to noise.
    - **Structural Auditing**: Align the Structure Tree and Tagged PDF metadata with ISO 32000-2 requirements.
- **State**: The document becomes a "perfectly specified tree." Temporary data is discarded (Precipitation), leaving only the "Normalization Recipe."

### Phase 3: Interpretation (Execution Layer)
- **Responsibility**: Transform normalized objects into a stateless sequence of drawing commands.
- **Actions**:
    - **On-demand Sublimation**: Instantaneously re-decompress and re-reconstruct data based on the Phase 2 recipes.
    - **Deterministic Mapping**: Apply pre-calculated mapping tables to content stream bytes without heuristic guesswork.
    - **Zero-Guesswork Execution**: Execute the atomic IR commands produced in Phase 2. No implicit state-changing logic is performed during this phase.
- **State**: A linear stream of drawing operators and GIDs.

### Text Layout & Coordinate Calculation (ISO 32000-2 Clause 9.3)
- **Coordinate Space Invariance**: Text is positioned in Text Space, which is transformed to User Space via the Text Matrix (`Tm`) and then to Device Space via the `CTM`.
- **Writing Mode (WMode) Determinism**:
    - **Horizontal (`WMode=0`)**: The displacement vector is `(w0, 0)`. Advance moves along the X-axis (typically rightwards).
    - **Vertical (`WMode=1`)**: The displacement vector is `(0, w1y)`. Advance moves along the Y-axis (typically downwards, as standard CJK `w1y` is negative).
- **Secondary Parameter Mapping**:
    - **Text Rise (`Ts`)**: Specifies a displacement perpendicular to the baseline.
        - Horizontal: Moves along the **Y-axis**.
        - Vertical: Moves along the **X-axis** (critical for correct Ruby/Furigana positioning).
    - **Horizontal Scaling (`Tz`)**: Scales the glyph and its advance along the writing direction.
        - Horizontal: Applies to the **X-axis** of the glyph and the advance.
        - Vertical: Applies to the **Y-axis** of the glyph and the advance (resulting in vertical compression/expansion).
- **Advance Calculation Formulas**:
    - **Horizontal**: `Advance_x = ((w / 1000) * Tfs + Tc + Tw) * (Tz / 100)`
    - **Vertical**: `Advance_y = ((w1y / 1000) * Tfs * (Tz / 100)) - Tc - Tw`
    - *Note*: Vertical advances subtract `Tc` and `Tw` to increase the downward gap, as $w_{1y}$ is natively negative.

### Phase 4: Rendering (Output Layer)
- **Responsibility**: Visual rasterization or conversion.
- **Actions**:
    - **Pure SFNT Rendering**: Pass reconstructed SFNT buffers and exact GIDs to the backend (e.g., `skrifa`).
    - **Zero-Fallback**: System font fallback is prohibited for embedded resources; fidelity must be 100%.

---

## 3. Memory Strategy: Sublimation
To balance correctness and memory efficiency, Ferruginous uses a "Sublimation" cycle:
1.  **Solid (Raw)**: Objects exist as compressed bytes in the arena.
2.  **Gas (Sublimated)**: Objects are expanded into analyzed, structured, and reconstructed forms during Phase 2/3.
3.  **Precipitation**: Once analysis or reconstruction is complete, the expanded or raw binary data is discarded (e.g., releasing `FontResource::data` once `reconstructed_data` is ready), reverting to the Solid state plus the associated "Normalization Recipe."
