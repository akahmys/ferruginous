# Ferruginous: High-Performance PDF 2.0 Engine & GUI Editor

**Ferruginous** is an ISO 32000-2:2020 compliant PDF processing platform. It is a **reference-grade PDF 2.0 toolkit** designed for the era of human-AI co-creation, utilizing a deterministic, hardware-accelerated architecture to provide professional-grade PDF analysis, normalization, and rendering.

The project is governed by the **RR-15 (Reliable Rust-15)** safety protocol, ensuring memory safety and deterministic behavior in mission-critical environments.

---

## 🎯 Vision & Philosophy

*"Proof over inference. Reliability over speed. Essence over prototyping."*

Ferruginous is more than just a library; it is a "Foundation of Trust" for the PDF ecosystem. Our "North Star" goals guide every commit:

- **Absolute Compliance**: Reaching the "Truth" of ISO 32000-2:2020 with a 100% pass rate in standard test suites.
- **Unbreakable Architecture**: Leveraging Rust 2024 and strict protocols (RR-15, HDD) to build a shield against memory corruption and logical regressions.
- **Superior Interaction**: Providing a CAD-grade visual experience through Vello/GPU, treating complex CJK (Japanese) vertical writing as a first-class citizen.
- **AI-Native Integrity**: Built from the ground up to be readable, editable, and verifiable by both humans and AI agents.

---

## 🤖 Human-AI Co-creation Symphony

Ferruginous is developed through a deep partnership between a human developer and the AI agent **Antigravity (Gemini 2.0 / 3 Flash)**. 

- **Autonomous Implementation**: Antigravity handles the bulk of the implementation, including the design and enforcement of its own safety protocols (`RR-15`).
- **Headless Verification**: A dedicated visual regression module allows Antigravity to "self-inspect" rendering outputs via automated JPEG/PNG comparisons, ensuring high-fidelity results without human intervention.
- **MCP Integration**: The **Model Context Protocol** is a first-class citizen, enabling AI agents to directly interact with PDF structures as a native tool.
- **ELM (External Long-Term Memory)**: Real-time persistence of state and design decisions prevents context loss and ensures continuity across long-running development cycles.

---

## 🛡️ Core Architecture: The Ingest & Refinement Pipeline

Ferruginous implements a multi-pass "Active Ingestion" pipeline that converts physical PDF structures into a high-purity internal data model.

1.  **Physical Parsing**: Utilizes `lopdf` as a gateway for initial object extraction.
2.  **Typed Arena Storage (`PdfArena`)**: Objects are decoupled from physical offsets and stored in a type-safe arena using generational `u32` handles.
3. **Active Normalization**:
    *   **Unicode-Native Pipeline**: Context-aware string re-encoding (`Byte` -> `UTF-8`) during ingestion to eliminate mojibake.
    *   **Color Harmonization**: Strict ICC profile application via **moxcms**.
    *   **Metadata Sublimation**: Conversion of legacy Info dictionaries into XMP-compliant streams.
    *   **Structural Hardening**: Active remediation of logical structure tags for **ISO 14289-2 (PDF/UA-2)** compliance.

---

## 💎 Ferruginous SDK (Decomposable Layers)

We conquer PDF complexity by decomposing it into independently verifiable layers:

1.  **`ferruginous-core` (The Grammar)**: Basic types, zero-copy lexical analysis, and COS parsing.
2.  **`ferruginous-doc` (The Structure)**: XRef systems, object streams, and the document catalog.
3.  **`ferruginous-render` (The Expression)**: Coordinate transformations, graphics states, and hardware bridges.
4.  **`ferruginous-sdk` (The Conductor)**: A high-level API that safely integrates all underlying layers.
5.  **`ferruginous-mcp` (The Interface)**: AI-native tools for document diagnosis and repair.

---

## 🚀 Hardware-Accelerated Rendering

The rendering pipeline is designed for resolution-independent, low-latency visual output.

- **Backend**: **Vello** (Compute-centric vector graphics renderer).
- **GPU API**: **WGPU** (Cross-platform WebGPU implementation).
- **Architecture**: Async `DisplayList` model optimized for CAD-grade viewing and modeless context UX.

---

## 🛠️ Compliance & Specifications

- **ISO Standards**: Optimized for ISO 32000-2:2020 and **ISO 14289-2 (PDF/UA-2)**.
- **Audit Protocol**: Adopts the **Matterhorn Protocol** for rigorous accessibility validation.
- **Limitation Policy**: "Liberal Read, Strict Write" — maximizes compatibility for ingestion while enforcing 100% specification compliance for output.

---

## 📦 Project Structure

- **`ferruginous`**: The "Sentinel" GUI application — CAD-grade viewing and editing.
- **`fepdf`**: Professional diagnostic and remediation CLI toolkit.
- **`ferruginous-sdk`**: Primary library for standard-compliant PDF manipulation.
- **`ferruginous-core`**: The foundational `PdfArena` and normalization logic.
- **`ferruginous-render`**: GPU-accelerated drawing backend.
- **`ferruginous-mcp`**: MCP server for AI-driven PDF management.

---

## 🛠️ Command Line Toolkit (`fepdf`)

`fepdf` is the professional CLI companion for Ferruginous, designed for batch processing, document auditing, and structural remediation.

### Key Subcommands
- **`inspect`**: Audit compliance (UA-2) and visualize document structure.
- **`upgrade`**: Modernize legacy PDFs to ISO 32000-2 (PDF 2.0).
- **`merge` / `split`**: High-fidelity document manipulation using iterative `ObjectCloner`.
- **`sign`**: Apply PAdES-compliant digital signatures with robust `ByteRange` patching.
- **`repair`**: Salvage corrupted PDF files using the hardened parser.
- **`credits`**: Display open-source attributions and licenses.

### Unified Optimization & Ingestion Flags
All processing commands share a consistent set of professional-grade options:

#### Optimization (Writing)
- **`--compress`**: Enable **FlateDecode** stream compression for minimal file size.
- **`--vacuum`**: Remove all unreachable objects (structural garbage collection).
- **`--linearize`**: Enable **Fast Web View** (ISO 32000-2 Annex F) with Hint Table generation.
- **`--strip`**: Remove descriptive metadata for anonymity and size reduction.
- **`--obj-stm`**: Use **Object Streams** for high-density compression (PDF 1.5+).
- **`--password <PWD>`**: Apply document open encryption.

#### Ingestion (Reading)
- `--no-refinement`: Skip the active 2-pass UTF-8 normalization.
- `--relaxed-color`: Use a lenient color validation policy.

---

---

## ⚙️ Development Requirements

- **Toolchain**: Rust 1.94+ / Edition 2024.
- **Automated Verification**: Run `make verify` to execute the full RR-15 compliance audit and visual regression suite.

---

## 📜 License

- **MIT License** / **Apache-2.0**
- ISO 32000-2:2020 Compliant Technical Baseline.
