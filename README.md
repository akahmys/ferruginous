# Ferruginous Rebirth

**Ferruginous** is a high-precision, ISO 32000-2 compliant PDF 2.0 toolkit built with Rust. This "Rebirth" project is a total reconstruction focused on "Absolute Compliance" and "Reliable Rust-15" (RR-15) engineering.

### 🌟 Latest Achievement (2026-04-19)
- **Phase 9 Completion**: Hardened **Security & Digital Signatures (PAdES)**. Implemented AES-256 encryption, ECDSA/RSA signature verification, and full LTV (DSS) support with DocMDP/FieldMDP modification detection.
- **Phase 8 Completion**: Advanced **Typography & Font Resolution**. Implemented token-based CMap parsing, CIDFont metrics support, and Standard 14 font fallbacks.
- **Audit-Ready Tooling**: Integrated `verify_signatures` into the MCP layer for automated cryptographic auditing.

## 🏗️ Architecture

The toolkit is organized into six specialized layers:

1.  **`ferruginous-core`**: Foundational layer. zero-copy Lexer, Recursive Descent Parser, and unified Object model.
2.  **`ferruginous-doc`**: Structural layer. XRef streams, incremental updates, and thread-safe caching.
3.  **`ferruginous-render`**: Visual layer. Powered by **Vello** (WGPU) for high-fidelity vector graphics.
4.  **`ferruginous-bridge-legacy`**: Validation layer. Modular adapter for `lopdf` with repair auditing.
5.  **`ferruginous-sdk`**: Orchestration layer. Type-safe High-level API for application developers.
6.  **`ferruginous-mcp`**: Intelligence layer. Model Context Protocol for AI-driven auditing.

## 🛡️ Reliable Rust-15 (RR-15)

Every line of code adheres to the RR-15 constraints:
- **Context Guard**: Mandatory injection of resolvers and resources (Clause 16).
- **No-Panic**: `unwrap()` and `expect()` are prohibited in non-test code.
- **SafetyIsolation**: Zero `unsafe` blocks in the core processing layers.
- **Stack Safety**: Recursion depth is strictly limited (16-level limit for XObjects).

## 🚀 Getting Started

### Prerequisites
- Rust 1.94+
- Cargo

### Building
```bash
cargo build --workspace
```

### Testing
```bash
cargo test --workspace
```

## 📜 License
Dual-licensed under MIT and Apache 2.0.
