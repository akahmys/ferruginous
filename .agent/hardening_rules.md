# Hardening Rules & Lessons Learned (Phase 6b)

These rules capture the architectural hardening requirements identified during the transition from prototype to production engine.

## 1. Modern PDF Baseline
- **Rule**: All PDF processing logic MUST support modern structural features by default.
- **Criteria**:
    - **XRef Streams**: Support for `/Type /XRef` stream parsing is mandatory.
    - **Incremental Updates**: Support for FOLLOWING the `/Prev` chain in trailers to find the most recent object versions is mandatory.
    - **Object Cache**: High-performance caching (using `parking_lot::RwLock`) MUST be used for frequent object resolution (especially Resources and Fonts).

## 2. Environment Stability
- **Rule**: Automated tools and scripts MUST reference the absolute path to `cargo` if the interactive shell PATH is unreliable.
- **Implementation**:
    ```bash
    export PATH="$HOME/.cargo/bin:$PATH"
    ```
- **Goal**: Prevent "command not found" (127) errors in agentic execution environments.

## 3. Resource Intelligence
- **Rule**: Attribute resolution for Page objects MUST follow ISO 32000-2 Clause 7.7.3.3 (Inheritance).
- **Mandate**: Explicitly track the parent chain (Page Tree hierarchy) during traversal to resolve `/Resources`, `/MediaBox`, and `/Rotate` correctly.

## 4. Performance & Concurrency
- **Rule**: Prefer `parking_lot` over `std::sync` for high-performance concurrent data structures.
- **Reasoning**: Avoids OS-level overhead and provides better performance in the high-frequency object resolution required for rendering.

## 5. Identifier Safety (Rust 2024+)
- **Rule**: Avoid using keywords that are reserved in modern Rust editions (e.g., Rust 2024) as identifiers.
- **Criteria**:
    - **`gen`**: Do NOT use `gen` as a variable or parameter name (it is reserved for generators). Use `generation` or `r#gen` instead.
    - **Future-Proofing**: Proactively audit identifiers against upcoming reserved keywords to prevent breaking changes during library evolution.

## 6. Security & Encryption Compliance
- **Rule**: All security handlers MUST strictly adhere to the decryption exclusions defined in ISO 32000-2 Section 7.6.2.
- **Mandate**:
    - **No Decryption for Containers**: Object Streams (`/ObjStm`) and Cross-Reference Streams (`/XRef`) must NEVER be decrypted, even in an encrypted document.
    - **AESV3 Partial Blocks**: Revision 6+ decryption must handle non-16-byte blocks using the ECB-mask XOR method rather than padding.
    - **Revision 6 Key Salt**: The FEK derivation for Revision 6 must use the Key Salt at `U[40..48]`, NOT the validation salt.
## 7. Structural Logic & Validation
- **Rule**: Validation logic MUST be deterministic and strictly decoupled from the repair engine.
- **Criteria**:
    - **Predicate Purity**: Arlington predicates MUST be parsed and evaluated against the document state without side effects.
    - **Heuristic Labeling**: Tagged PDF repair should favor "conservative labeling" (e.g., defaulting to `P` tags) to avoid mischaracterizing structural hierarchy.
    - **Encoding Fidelity**: Always prioritize `/Differences` and `/CMap` over heuristic character mapping during structural inference.
