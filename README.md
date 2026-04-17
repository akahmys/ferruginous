# Ferruginous Rebirth

**Ferruginous** is a high-precision, ISO 32000-2 compliant PDF 2.0 toolkit built with Rust. This "Rebirth" project is a total reconstruction focused on "Absolute Compliance" and "Reliable Rust-15" (RR-15) engineering.

### 🌟 Latest Achievement (2026-04-18)
Successfully rendered a sample PDF from the **PDF Association Sample Suite** to PNG using the `ferruginous-sdk` and `vello` backend, resolving a critical COS parser lookahead bug.

## 🏗️ Architecture

The toolkit is organized into five specialized layers:

1.  **`ferruginous-core`**: The foundational layer. Includes a zero-copy Lexer, Recursive Descent Parser, and standard Object types with unified numeric access.
2.  **`ferruginous-doc`**: The structural layer. Handles XRef tables/streams, incremental updates, thread-safe object caching, and attribute inheritance.
3.  **`ferruginous-render`**: The visual layer. High-fidelity rendering bridge powered by **Vello** (WGPU). Implements Path Construction, Clipping, and Styled Strokes.
4.  **`ferruginous-bridge-legacy`**: The validation layer. Modular adapter for `lopdf`, enabling differential testing and legacy repair auditing.
5.  **`ferruginous-sdk`**: The orchestration layer. High-level API for document loading and rendering. (Active Development)
6.  **`ferruginous-mcp`**: The intelligence layer. Model Context Protocol for AI-driven document analysis and compliance auditing. (Functional / v1.0 Server)

## 🛡️ Reliable Rust-15 (RR-15)

Every line of code in this project adheres to the RR-15 constraints:
- **No-Panic**: `unwrap()` and `expect()` are strictly prohibited.
- **Safety Isolation**: Zero `unsafe` blocks in the core/document layers.
- **Stack Safety**: Recursion depth is strictly limited and monitored.
- **Deterministic**: 100% bit-perfect consistency through `BTreeMap` and fixed iteration orders.

## 🚀 Getting Started

### Prerequisites
- Rust 1.83+
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
