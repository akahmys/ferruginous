# Ferruginous Rebirth

**Ferruginous** is a high-precision, ISO 32000-2 compliant PDF 2.0 toolkit built with Rust. This "Rebirth" project is a total reconstruction focused on "Absolute Compliance" and "Reliable Rust-15" (RR-15) engineering.

### 🌟 Latest Achievement (2026-04-19)
- **v1.0 Production Readiness (Phase 11)**: Completed the ecosystem transition. Implemented a high-performance CLI tool (`info`, `render`, `audit`, `extract`), introduced the PDF Physical Serializer (`writer.rs`), and established the WASM bridge for web integration.
- **Standards & Compliance (Phase 10)**: Finalized conformance auditing for PDF/A-4, PDF/X-6, and PDF/UA-2. Enhanced the MCP layer with automated structural auditing.
- **Security & Digital Signatures (Phase 9)**: Hardened **Security & Digital Signatures (PAdES)** with AES-256 encryption and full LTV (DSS) support.

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
