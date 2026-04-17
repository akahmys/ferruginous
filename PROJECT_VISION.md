# Ferruginous Project Vision: Rebirth

**Ferruginous** is more than just a PDF library. It is a **reference-grade PDF 2.0 (ISO 32000-2:2020) toolkit** designed for the era of human-AI co-creation.

## 1. The "North Star" Goals

### I. Absolute Compliance (Reaching the Truth)
- **Standard**: Full compliance with ISO 32000-2:2020.
- **Verification**: 100% pass rate in the [PDF Association Sample Suite](https://github.com/pdf-association/pdf-issues).
- **Diagnostics**: Eliminate guess-based coding; enforce specification-first development using `pdf-spec-mcp`.

### II. Unbreakable Architecture (The Unwavering Shield)
- **Language**: Rust (Edition 2024 / MSRV 1.83+).
- **Protocols**: Strict application of **RR-15 (Reliable Rust 15)** and **HDD (Harness-Driven Development)**.
- **Data Model**: Immutable, zero-copy design utilizing `bytes::Bytes` and `Arc<T>`. Balancing high performance with total memory safety.

### III. Excellence in Vision & Interaction (Sentinel)
- **Vello/GPU**: A modern vector graphics pipeline that bypasses traditional CPU bottlenecks.
- **Sentinel UI**: A premium design system featuring HSL tokens, micro-interactions, and modeless context UX.
- **Superior Text Precision**: Perfect implementation of multi-byte characters and vertical writing (including Japanese) as "first-class features," not afterthoughts.

### IV. Human-AI Co-creation Symphony (The Evolving Process)
- **Antigravity**: AI agents autonomously handle the bulk of the implementation, while humans focus on strategic direction and milestone reviews.
- **Headless Verification**: A dedicated module for exporting rendering results to JPEG/PNG, allowing AI agents (Antigravity) to autonomously perform visual regression tests and "self-inspection" using the `view_file` tool.
- **MCP Integration**: The **Model Context Protocol** is integrated as a first-class citizen, enabling AI agents to directly "read, write, and edit" PDF data.
- **ELM (External Long-Term Memory)**: Real-time persistence of thoughts and state to physically prevent "context loss" for AI agents.

## 2. Technical Philosophy: "Simple yet Robust"

We conquer the massive complexity of the PDF standard by decomposing it into five independently verifiable layers:

1. **`ferruginous-core`**: The "Grammar" of PDF. Basic types, zero-copy lexical analysis, and COS parsing.
2. **`ferruginous-doc`**: The "Structure" of PDF. XRef systems, object streams, and the document catalog.
3. **`ferruginous-render`**: The "Expression" of PDF. Coordinate transformations, graphics states, and rendering bridges.
4. **`ferruginous-sdk`**: The "Conductor" of PDF. A high-level API that safely integrates all underlying layers for the user.
5. **`ferruginous-mcp`**: The "Interface" of PDF. An MCP server implementation providing AI-native tools for document diagnosis and repair.

## 3. Final Destination: "Sentinel v2.0"

The final outcome is a desktop application ("Sentinel") that provides a CAD-grade viewing and editing experience. Its core SDK will serve as a "foundation of trust" even in professional printing and archiving workflows (PDF/X-6, PDF/A-4).

---
*"Proof over inference. Reliability over speed. Essence over prototyping."*
