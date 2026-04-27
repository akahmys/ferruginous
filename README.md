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

### AI-Native Design (MCP & ELM)
The architecture is designed to be fully transparent to AI agents:
- **Autonomous Implementation**: Antigravity handles the bulk of the implementation, including the design and enforcement of its own safety protocols (`RR-15`).
- **Headless Verification**: A dedicated visual regression module allows Antigravity to "self-inspect" rendering outputs via automated JPEG/PNG comparisons.
- **MCP Integration**: The **Model Context Protocol** is a first-class citizen, enabling AI agents to directly interact with PDF structures as a native tool.
- **ELM (External Long-Term Memory)**: All stateful decisions and design intents are recorded in `.antigravity/session/` to survive session resets and ensure continuity.

---

## 🛡️ Core Technical Architecture: The "Active Ingestion" Pipeline

Ferruginous does not simply "read" a PDF; it **ingests** it. This process converts the physical, often inconsistent byte-stream of a PDF file into a high-purity, semantically indexed internal model.

### 1. The Multi-Pass Pipeline

The ingestion process is divided into distinct, non-overlapping phases:

#### **Pass 0: Physical Normalization (The Guard Phase)**
- **Scope**: Raw `lopdf::Document` objects.
- **Objective**: Ensure the data is plain, reachable, and safe for semantic indexing.
- **Operations**:
    - **Recursive Decryption**: A stack-based (Rule 6) walk of all objects to decrypt strings and streams.
    - **Security Handler Removal**: Stripping the `/Encrypt` dictionary from the trailer to prevent Acrobat Error 135.
    - **Physical Repair**: Fixing broken XRef offsets and object numbers before they reach the `PdfArena`.
- **Naming Convention**: `perform_pass_0_<action>`.

#### **Pass 1: Structural Ingestion (The Arena Phase)**
- **Scope**: Mapping physical objects to generational `Handle<Object>`.
- **Objective**: Decouple the document structure from its physical byte-offsets.
- **Operations**:
    - Object stream expansion.
    - Generation of unique IDs for every object.
    - Deduplication of common resource objects.

#### **Pass 2: Semantic Refinement (The Truth Phase)**
- **Scope**: Typed interpretation of object dictionaries.
- **Objective**: Normalize the content to ISO 32000-2:2020 standards.
- **Operations**:
    - **Unicode-Native Pipeline**: Context-aware string re-encoding (`Byte` -> `UTF-8`) to eliminate mojibake.
    - **Color Sublimation**: Strict ICC profile application via **moxcms**.
    - **Metadata Extraction**: Converting legacy `/Info` to XMP metadata.
    - **Structural Hardening**: Active remediation of logical structure tags for **ISO 14289-2 (PDF/UA-2)** compliance.

### 2. Memory & Safety: `PdfArena`

`PdfArena` is the backbone of the Ferruginous SDK. It utilizes a generational arena to manage object lifetimes.

- **Handles vs. Pointers**: All object references are `Handle<Object>` (a `u32` index and a generation count). This prevents "use-after-free" and makes the entire document structure easily serializable and AI-inspectable.
- **RR-15 Compliance**: 
    - **Rule 6 (No Recursion)**: All traversals of the PDF object graph must use an explicit stack (`Vec`) to prevent stack overflow on deeply nested documents.
    - **Rule 10 (Determinism)**: Iteration over objects and metadata generation is deterministic to ensure bit-perfect output and reliable digital signatures.
    - **Rule 11 (Error Transparency)**: Zero use of generic `String` errors. All failures use structured Enum variants with mandatory context (e.g., `pos`, `context`).
    - **Rule 19 (Unbounded Recursion Guard)**: Strict enforcement of depth limits (e.g., max 32 levels) for tree-like structures like the Page Tree and Structure Tree.

---

## 🔐 Security & Compliance

### Encryption Handling
Ferruginous implements custom security handlers for PDF 1.4-2.0, focusing on Adobe fidelity:
- **AES-128 (Revision 4)**: Used in PDF 1.6. Requires specific key padding and alignment with MD5-based derivation.
- **AES-256 (Revision 5/6)**: Used in PDF 1.7/2.0. Implements the SHA-256 based key derivation.
- **Pass 0 Decryption**: Ensures the internal engine only ever sees plaintext, reducing the surface area for logical bugs.

### ISO Standards & Audit
- **Specifications**: Optimized for **ISO 32000-2:2020** and **ISO 14289-2 (PDF/UA-2)**.
- **Audit Protocol**: Adopts the **Matterhorn Protocol** for rigorous accessibility validation.
- **Limitation Policy**: "Liberal Read, Strict Write" — maximizes compatibility for ingestion while enforcing 100% specification compliance for output.

---

## 💎 Ferruginous SDK (Decomposable Layers)

We conquer PDF complexity by decomposing it into independently verifiable layers:

1.  **`ferruginous-core` (The Grammar)**: Foundational `PdfArena`, normalization logic, and typesafe handles.
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

## 📦 Project Structure

- **`ferruginous`**: The "Sentinel" GUI application — CAD-grade viewing and editing.
- **`fepdf`**: Professional diagnostic and remediation CLI toolkit.
- **`ferruginous-sdk`**: Primary library for standard-compliant PDF manipulation.
- **`ferruginous-core`**: The engine's heart: `PdfArena` and ingestion pipeline.
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

### Optimization & Ingestion Flags
- **`--compress`**: Enable **FlateDecode** stream compression.
- **`--vacuum`**: Remove all unreachable objects (GC).
- **`--linearize`**: Enable **Fast Web View** (Annex F).
- **`--obj-stm`**: Use **Object Streams** for high-density compression (PDF 1.5+).
- **`--no-refinement`**: Skip the active 2-pass normalization.

---

## ⚙️ Development Requirements

- **Toolchain**: Rust 1.94+ / Edition 2024.
- **Verification**: Run `make verify` to execute the RR-15 compliance audit and visual regression suite.

---

## 📜 License

- **MIT License** / **Apache-2.0**
- ISO 32000-2:2020 Compliant Technical Baseline.
