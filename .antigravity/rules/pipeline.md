# The Ferruginous Processing Pipeline

This document defines the canonical five-phase pipeline for transforming a physical PDF file into a compliant, rendered, and semantically consistent document.

## 1. Overview: Normalization-at-Load
Ferruginous operates on the principles of **"Normalization-at-Load"** and **"Delayed Normalization."** The objective is to resolve all ambiguities, inheritance chains, and non-standard data structures at the earliest possible stage, while **preserving raw high-fidelity data** (e.g., original color spaces) within the Intermediate Representation (IR) until the final rendering or serialization stage. This ensures that downstream processes remain purely deterministic, lossless, and free of side effects.

---

## 2. The Five Phases

### Phase 1: Ingestion & Decryption (Physical Layer)
*   **Responsibility**: Resolve physical addressing, object mapping, and access control.
*   **Actions**:
    *   **XRef Resolution & Inhalation**:
        *   Map physical `ObjectId` pairs to logical `Handle<Object>` pointers, decoupling the logical document structure from physical file offsets.
        *   Heuristically repair corrupted or fragmented XRef tables during the inhalation process.
    *   **Pass 0 Decryption**:
        *   **Iterative Traversal**: Decrypt all strings and streams using a stack-based, non-recursive walk to mitigate stack overflow risks in deeply nested object graphs.
        *   **Fidelity Cleanup**: Explicitly remove the `/Encrypt` entry from the trailer post-decryption to satisfy strict ISO 32000-2 requirements and prevent legacy viewer errors (e.g., Error 135).
*   **State**: Objects reside in the `PdfArena` as a flat, addressable set of logical handles.

### Phase 2: Normalization & Pre-Analysis (Logical Layer)
*   **Responsibility**: Transform "Dirty" source data into "Ideal" PDF 2.0 structures.
*   **Actions**:
    *   **Resource Inheritance Flattening**:
        *   Traverse the Page Tree's `Parent` hierarchy to collect and flatten `Resources` (Fonts, XObjects, ColorSpaces).
        *   Assign a self-contained resource map to every content stream, enabling **Context-free Rendering** without upward lookups.
    *   **Content Stream Sublimation (Body Normalization)**:
        *   **Operator Atomicity**: Decompose complex operators with implicit side effects (e.g., `TD`, `"`, `'`) into sequences of atomic IR commands (e.g., `SetTextLeading`, `MoveToNextLine`, `ShowText`).
        *   **Path Integrity & Termination**: Explicitly preserve the `EndPath` (`n`) operator. Discarding `n` triggers "path construction leakage," where clipping paths or construction segments are erroneously inherited by subsequent painting operations, manifesting as "black mask" artifacts.
        *   **Writing Mode Injection**: Inject explicit `SetWritingMode` commands into the IR stream during font selection (`Tf`). This flattens the Writing Mode state and ensures deterministic layout for documents with mixed horizontal/vertical streams.
        *   **High-Fidelity Color Preservation**: Maintain original color space semantics (Gray, RGB, CMYK, Lab) throughout the IR pipeline. Downgrading to RGB at the sublimation stage is prohibited as it loses device-specific color profile context and prevents accurate color management in modern rendering backends.
        *   **Corruption Resurrection**: Detect and sanitize content streams containing non-standard "leaked" data (e.g., development debug logs).
    *   **Heuristic Sanitization (Visual Cleanup)**:
        *   **Structural Bar Suppression**: Identify large horizontal rectangles (`Rect`) at the extreme vertical bounds (`y > 700` or `y < 50`) that lack structural or semantic purpose. Suppress these by converting the painting operator to a no-op path termination (`n`).
    *   **Font SFNT Modernization (Structural Normalization)**:
        *   **Format Identification**: Strict reliance on binary signatures (Magic Bytes) rather than dictionary subtypes.
            *   `OTTO`, `00 01 00 00`: SFNT (OpenType/TrueType)
            *   `01 00 04`: CFF v1.0 (Naked CFF)
            *   `02 00 08`: CFF v2.0
            *   `80 01`: Type 1 PFB (Binary)
            *   `%!`: Type 1 PFA (ASCII)
        *   **Precipitation (SFNT Wrapping)**: Encapsulate Naked CFF or Type 1 outlines into a minimal Virtual OpenType (SFNT) container. This unifies all font types for modern rendering backends (e.g., `skrifa`).
        *   **Metric & Mapping Injection**: Authoritatively inject `hmtx`, `OS/2`, and synthesized `cmap` tables into the reconstructed binary.
        *   **Metrics Branching**: Explicitly distinguish between `/Widths` (Simple fonts) and `/W` (CIDFonts). Standalone `CIDFontType0/2` resources MUST utilize `/W` parsing to prevent the 1000-unit default width regression.
    *   **Handle Stability (RR-15 Hardening)**:
        *   **Object-Centric Modeling**: Persistent structural components (Catalog, Page, StructTreeRoot) MUST utilize stable `Handle<Object>` references. Direct storage of volatile `DictHandle` is prohibited.
        *   **Late-bound Dictionary Resolution**: Resolve `Handle<Object>` to `DictHandle` at the point of access. This ensures reference validity even if the underlying memory is reallocated during a `ParallelRefinery` pass.
    *   **Semantic Mapping & Character Resolution Chain**:
        *   **Bridged CMap Synthesis**:
            *   **Non-CJK (Western) Logic**: Prioritize linguistic metadata over structural PDF claims.
                *   Priority 0: Physical truth (Internal Encoding/Charset).
                *   Priority 0.5: Unicode Name matching (e.g., 'A' -> "uni0041"), bypassing "lying" subsetted mappings.
                *   Priority 1: Internal SFNT Unicode `cmap` tables.
                *   Priority 3: Identity fallback (CID as GID) as a last resort.
            *   **CJK Logic**: Prioritize structural CID mapping to preserve document-specific glyph selection (variants, IVS).
                *   Priority 0: Structural truth (**`Char Code` -> `CMap` -> `CID` -> `GID`**).
                *   Priority 1: Internal SFNT `cmap` if CID resolution fails.
*   **State**: The document exists as a "Perfectly Specified Tree." Physical transients are discarded, leaving an immutable **Normalization Recipe**.

### Phase 3: Interpretation (Execution Layer)
*   **Responsibility**: Transform normalized objects into a stateless sequence of drawing commands.
*   **Actions**:
    *   **On-demand Sublimation**: Re-decompress and re-reconstruct data on-the-fly based on Phase 2 recipes.
    *   **Stateless Execution**: Execute the atomic IR commands produced in Phase 2. No heuristic guesswork or implicit state mutations are permitted during this phase.
    *   **Exhaustive Operator Dispatching (Rule 5 Hardening)**: The interpreter MUST utilize exhaustive pattern matching for the `Command` IR enum. The use of wildcards (`_`) in the primary dispatch loop is prohibited. Every variant—including XObjects, marked content, and Type 3 metrics—must be explicitly routed to its corresponding handler to prevent "silent state loss" where operators are parsed but never executed.
*   **Coordinate System Decoupling**:
    *   **Baseline Transform (`initial_transform`)**: The immutable transform from PDF User Space (Points) to Device Space (Pixels). Handles MediaBox translation, Y-flipping, and DPI scaling.
    *   **Execution CTM**: Maintains a pure, PDF-compliant Y-up `ctm` within each graphics state.
    *   **Composition**: `BackendTransform = initial_transform * current_ctm`. This prevents double-inversion and isolates the interpreter from device-specific constraints.

### Phase 4: Rendering (Output Layer)
*   **Responsibility**: High-fidelity visual rasterization.
*   **Actions**:
    *   **Pure SFNT Pipeline**: Pass reconstructed SFNT buffers and exact GIDs to the hardware-accelerated backend.
    *   **Zero-Fallback Policy**: System font fallback is strictly prohibited for embedded resources; visual fidelity must be absolute.

### Phase 5: Serialization & Resurrection (Refinery Export)
*   **Responsibility**: Reify the normalized IR back into a physical PDF 2.0 file.
*   **Lossless Reversibility**:
    *   **State Preservation**: All IR commands (e.g., `SetFillColor`, `SetStrokeColor`) MUST be mapped back to their canonical PDF operators (`rg`, `RG`, `g`, `G`, `k`, `K`). Omissions here lead to "Default-to-Black" regressions.
    *   **Raw Operator Passthrough**: Operators captured as `RawOperator` (e.g., `n`, `v`, `y`) must be emitted exactly as captured to preserve path logic and drawing order.
    *   **Compliance Verification**: The resulting PDF must pass iterative structural auditing for the target standard (e.g., PDF/UA-2).

---

## 3. Memory Strategy: The Sublimation Cycle
To optimize for both accuracy and memory throughput, Ferruginous utilizes a cyclical state model:

1.  **Solid (Raw)**: Objects exist as compressed, physical bytes within the `PdfArena`.
2.  **Gas (Sublimated)**: Objects are expanded into analyzed, structured, and reconstructed forms during processing.
3.  **Precipitation**: Post-analysis, expanded data is discarded, reverting the object to its Solid state while persisting the associated **Normalization Recipe** for future on-demand sublimation.
