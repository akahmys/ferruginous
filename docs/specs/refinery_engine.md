# Technical Specification: The Ferruginous Refinery Engine

## 1. Overview
Ferruginous is a "Refinery" type PDF engine designed to extract pure PDF 2.0 representations from legacy PDFs and optimize them for modern computing environments.

## 2. Core Components

### 2.1 PdfArena (Typed Arena Storage)
All PDF objects are decoupled from their physical locations and stored in a type-safe arena structure.
- **Data-Oriented Design**: Dicts, Arrays, and Streams are managed in independent memory pools to maximize cache efficiency.
- **Handle System**: Objects are cross-referenced by lightweight `u32`-based handles. Generational management bits ensure memory safety.

### 2.2 Refinery Pipeline (Refinement Process)
1. **Ingestion**: Deconstruct physical structures via `lopdf` and perform high-speed transfers to the arena.
2. **Normalization**: 
   - **Text Recovery**: Infer and convert text with missing character encodings to UTF-8 using `chardetng` and `encoding_rs`.
   - **Color Harmonization**: Normalize device-dependent colors to OutputIntents (ICC) using `moxcms` (Pure Rust).
   - **Metadata Scrubbing**: Consolidate legacy Info into XMP streams using `xmp-writer`.
3. **Validation**: Perform consistency checks based on the Arlington model and assign a `SafetyBitmask`.

## 3. Flagship GUI (`ferruginous`)
- **Rendering**: GPU-accelerated rendering via Vello, using normalized data on the arena as the direct source.
- **Asynchronous Design**: Ingestion and refinement are executed on background threads (Tokio/Rayon), maintaining GUI responsiveness.

## 4. Security and Signatures
- **PAdES Compliance**: Digital signature application and verification using `cms` and `x509-parser`.
- **Strict 2.0 Conversion**: Always perform a "Full Rewrite" during saving to forge a high-purity PDF 2.0 binary free of impurities.
