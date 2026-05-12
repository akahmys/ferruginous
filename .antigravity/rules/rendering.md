# Rendering Conventions

Design and implementation conventions for the Ferruginous rendering engine (Core, Render, SDK).

## 1. Text Metrics and Scaling
- **Decoupling Principle**: Generation of the Glyph Path and calculation of Layout (Advance/Metrics) must clearly separate scales.
    - **Path Scale**: `size / units_per_em` (using Font-specific UnitsPerEm).
    - **Metrics Scale**: `size / 1000.0` (using PDF standard 1000-unit system).
- **Rounding**: Manage precision strictly to prevent the accumulation of floating-point errors in layout calculations.

## 2. Coordinate Systems
- **Internal Sovereignty**: All internal logic (Interpreter, FontResource) must consistently use the **Positive Y = UP** coordinate system according to the PDF specification.
- **Conversion Boundary**: Invert the coordinate system (Positive Y = DOWN) ONLY in the layer immediately before sending data to the rendering device (e.g., Vello). Do not flip signs in intermediate pipeline layers.

## 3. Font Resource Normalization & Reconstruction
- **Normalization-at-Reconstruction**: All font-specific ambiguities (e.g., "Lying Identity," missing widths) MUST be resolved during the reconstruction phase. The resulting **Virtual OpenType (SFNT)** binary serves as the absolute single source of truth.
- **Propagation Obligation**: Mandatory inheritance of metadata (WMode, Encoding, ToUnicode) from Type 0 parents to CIDFont descendants during ingestion.
- **Metrics Integrity**: CIDFonts MUST be parsed using CID-specific metrics (`/W`), ensuring consistency between parent and descendant resources.
- **Path Integrity & Termination**: The `EndPath` (`n`) operator is critical for graphics state isolation. Discarding `n` during sublimation leads to "Path Leakage," where previous construction paths (e.g. clipping rectangles) are incorrectly painted by subsequent fill/stroke operators.

## 4. CMap and Encoding Hygiene
- **Isolation**: Each `FontResource` must have its own independent mapping table. "Rescue" logic (using common CMaps) is permitted only for clearly identified CJK fonts and must not have side effects (cache pollution).
- **Strict Parsing**: CMap parsing must accurately handle both literal strings (UTF-16BE) and hex notations.

## 5. Context Propagation Guard
- **Rule**: Interpretation of document data MUST enforce type-level provision of a `Resolver` and `ResourceStack`.
- **Late-bound Initialization**: The `ResourceStack` MUST be initialized using late-bound resolution (e.g., `Page::resources_handle()`). Storing or passing `DictHandle` from previous passes for stack initialization is prohibited.
- **Compliance Criterion**: Public high-level interpreters MUST NOT have default constructors that omit these dependencies.

## 6. High-Fidelity CJK Decoding
- **Boundary Precision**: Multi-byte character decoding (CMap) MUST accurately detect byte-length boundaries (1-byte vs 2-byte) based on the specific CMap's range definitions.
- **Fail-Safe Mapping**: If a character mapping is missing, the engine MUST fallback to a diagnostic placeholder (e.g., `.notdef`) and log the incident, rather than silently guessing or shifting indices.
- **WMode Fidelity**: Vertical writing (WMode=1) metrics MUST be applied strictly according to the CIDFont's W/W2 dictionaries.
