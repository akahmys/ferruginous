# Ferruginous: A Personal Quest for PDF 2.0 Fidelity

**Ferruginous** is an experimental, high-fidelity PDF processing platform engineered with Rust. It achieves **ISO 32000-2:2020** compliance through a deterministic, hardware-accelerated architecture designed to master the complexity of modern and legacy PDF structures.

The project strictly adheres to the **RR-15 (Reliable Rust-15)** hardening protocol—a mission-critical safety standard derived from aerospace principles to ensure memory safety, bit-perfect determinism, and absolute reliability.

**🚀 Status: Interactive GUI Editor & Multi-Language Support Fully Operational** — As of June 2026, the platform features a GPU-accelerated egui + wgpu + Vello interactive workspace supporting real-time CJK/Japanese text selection, accessibility tagging, CAD-grade path snapping/measurement (Caliper Tool), secure permanent redaction (Atomic Redaction Studio), and built-in English/Japanese UI localization.

---

## 🎯 Vision & Goals

*"Reliability over speed. Essence over prototyping."*

Ferruginous serves as a technical laboratory for exploring the boundaries of document technology. Our "North Star" objectives are:

- **Rigorous Compliance**: Pursuing the absolute "Truth" of ISO 32000-2:2020 through exhaustive implementation.
- **Hardened Architecture**: Leveraging Rust 2024 and the RR-15 protocol to eliminate regressions and runtime failures.
- **Precision Typography**: Delivering CAD-grade visual quality via **Vello** (GPU Compute), with a specialized focus on complex CJK (Japanese) and Type 3 font rendering.
- **AI-Native Design**: Maintaining a verifiable, high-context codebase optimized for both human architects and AI assistants.

---

## 🤖 Building with AI (Antigravity)

Ferruginous is developed through a sophisticated collaboration between human document architects and the **Antigravity** AI agent.

### AI-Native Engineering
The platform is built to excel in autonomous agentic environments:
- **Safety as Code**: Security and hardening protocols are programmatically enforced via AI-driven audits.
- **Visual Sincerity**: Integrated visual regression modules enable the engine to "self-inspect" rendering fidelity.
- **MCP Integration**: First-class implementation of the **Model Context Protocol**, allowing AI agents to perform direct structural diagnostics.
- **Stateful Continuity**: Design intents and lifecycle decisions are persisted in the `ELM` (Extended Long-Term Memory) to ensure seamless multi-session development.

---

## 🛡️ The Ingestion Pipeline: Normalization-at-Load

Ferruginous utilizes a multi-pass "Sublimation" pipeline to transform volatile physical bytes into high-purity logical structures.

### 1. The Normalization Process
- **Pass 0: Physical Normalization**
    - Recursive, stack-based decryption and XRef repair.
    - Mandatory removal of `/Encrypt` residuals to ensure absolute Acrobat compatibility.
- **Pass 1: Arena Ingestion**
    - Expansion of object streams and generation of stable **Arena Handles**.
    - Deduplication of resources and structural indexing.
- **Pass 2: Semantic Sublimation**
    - Unicode-native string re-encoding to eliminate legacy mojibake.
    - Path integrity restoration (EndPath `n` preservation) and color state harmonization.
    - Structural remediation for **ISO 14289-2 (PDF/UA-2)** compliance.

### 2. Safety Invariants: `PdfArena`
- **Handles over Pointers**: All references utilize `Handle<Object>` (ID + Generation), preventing use-after-free and ensuring AI-inspectability.
- **Deterministic Iteration**: Collection traversal is bit-perfect, eliminating non-determinism in the generated output.

---

## 🏛️ Project Ecosystem

- **`ferruginous`**: 
    - The flagship desktop interface.
    - Integrated **egui** and **wgpu** for 120fps canvas interaction, asynchronous document streaming, CAD measurement, Accessibility Tagging, and Atomic Redaction Studio.
- **`fepdf`**: 
    - The universal CLI toolkit for structural auditing, repair, and production-grade PDF 2.0 output.
- **`ferruginous-sdk`**: 
    - The high-level library providing secure, handle-based APIs for document manipulation.
- **`ferruginous-core`**: 
    - The engine core, featuring the `PdfArena`, ISO-compliant parsers, and the Pass 0 normalization layer.
- **`ferruginous-render`**: 
    - The GPU-accelerated rendering backend utilizing **Vello** for compute-shader-based path rasterization.
- **`ferruginous-mcp`**: 
    - The bridge for AI agents, implementing MCP servers for autonomous document diagnostics.
- **`ferruginous-wasm`**: 
    - WebAssembly bindings for running the Ferruginous engine in browser environments.
- **`ferruginous-macros`**: 
    - Internal procedural macros for compile-time generation and RR-15 validation helpers.

---

## 🛠️ CLI Toolkit (`fepdf`)

| Category | Command | Description |
| :--- | :--- | :--- |
| **Analyze** | `audit`, `info`, `text` | Structural diagnostics and metadata inspection. |
| **Manipulate**| `merge`, `split`, `rotate`, `repair`, `retag` | Logical document modification, recovery, and semantic re-tagging. |
| **Produce** | `upgrade`, `sign`, `render` | PDF 2.0 re-production, signatures, and GPU-accelerated output. |
| **Debug** | `dump`, `structure`, `stats`, `extract-font`, `trace-glyph` | Low-level object, font, and glyph mapping tracing and diagnostics. |

---

## ⚙️ Development Environment

- **Toolchain**: Rust 1.94+ (Edition 2024).
- **Verification**: Execute `make help` to see available tasks.
- **Structure**: Refer to [WS-01: Workspace Structure](.antigravity/rules/structure.md) for detailed directory organization.

---

## 📜 License

- **MIT License**
- Designed for technically sound compliance with the ISO 32000-2:2020 standard.
