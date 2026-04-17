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
