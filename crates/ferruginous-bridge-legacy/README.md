# ferruginous-bridge-legacy

This crate provides a bridge layer for handling legacy PDF 1.7 (and earlier) features, ensuring that the core Ferruginous SDK can focus exclusively on PDF 2.0 (ISO 32000-2:2020) compliance.

## Purpose

Ferruginous is designed as a modern, next-generation PDF engine. To maintain a pure PDF 2.0 implementation with high-performance GPU rendering (Vello), we offload "dirty" legacy compliance tasks to this bridge, including:

- **Legacy Parsing**: Handling non-strict PDF structures and broken files that require repair.
- **Normalization**: Converting legacy character encodings (e.g., Shift-JIS) to standardized UTF-8.
- **Legacy Decryption**: Supporting older encryption algorithms like RC4 and AES-128.

## Architecture

The bridge is designed around a **Zero-copy Data Model**. It utilizes `bytes::Bytes` for all internal operations, shared with the core SDK's loader.

1. **Load Phase**: Detects non-PDF 2.0 signatures and routes data to the bridge.
2. **Normalization Phase**: The bridge parses legacy objects and applies necessary transformations (character mapping, decryption).
3. **Delivery Phase**: Cleaned and normalized data is handed back to the SDK Core for rendering and logical structure analysis.

## License & Attributes

Parts of the internal object model and parser logic in this crate are based on [lopdf](https://github.com/J-F-Liu/lopdf).

- **License**: MIT (inherited from `lopdf` components)
- **Copyright**: Original work © J-F-Liu. Modifications for Ferruginous architecture © 2026 Ferruginous Project.

## Features

- [x] **Zero-copy Object Model**: Adapted `lopdf` internal structures to use `Bytes`.
- [x] **Shift-JIS Normalizer**: Safe normalization of Japanese strings using `encoding_rs`.
- [x] **RC4 & AES-128 Decryptors**: Correctly implemented block decryption for legacy protected files.
- [ ] **Object Stream Reconstruction**: (Planned) Restoration of legacy object streams into modern structures.
