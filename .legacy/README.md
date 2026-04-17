# Ferruginous: A PDF 2.0 Toolkit for Human-AI Co-creation

Ferruginous is an open-source PDF toolkit currently under development with the goal of ISO 32000-2 (PDF 2.0) compliance.
This project is an experimental personal project that tackles the PDF standard as a programming challenge utilizing the AI agent **Antigravity**.

## Project Overview

The most significant feature of this project is the collaborative development system between humans and AI.
The human (developer) focuses on determining basic policies and reviewing milestones, while **Antigravity autonomously handles the majority of the concrete implementation.** Furthermore, it employs a self-refining development process where Antigravity itself implements and revises its own safety protocols and governance rules.

The goal is to build a modern PDF environment that balances "unbreakable" implementation—maximizing Rust's safety—with lightweight performance powered by the [Vello](https://github.com/linebender/vello) GPU rendering engine.

Currently, the project is in **Phase 19** (High-End Compliance & Archiving). We have successfully implemented a **Legacy PDF Bridge** architecture and a **Zero-copy Data Model** using `bytes::Bytes`, ensuring high performance while maintaining the purity of the PDF 2.0 core engine.

### ✅ Implemented Features

- **Parsing & Rendering (Phases 1-4)**: Parsing PDF document structure and supporting GPU rendering via Vello.
- **Search & Unicode Extraction (Phase 5)**: Text extraction via Unicode restoration and full-text search.
- **Annotation & Editing (Phase 5)**: Adding highlight and rectangle annotations and incremental saving.
- **Forms & Layers (Phase 6)**: AcroForm input/saving and OCG layer control.
- **Advanced Graphics (Phase 7)**: ICC color management, transparency groups, and complex shading.
- **Security & Trust (Phase 8)**: AES-256 Rev 6 encryption, PAdES electronic signatures (LTV), and redaction.
- **Logical Structure & Accessibility (Phase 9)**: Tagged PDF parsing, RoleMap/ClassMap resolution, and consistency verification UI.
- **Multimedia & Quality Assurance (Phase 10)**: RichMedia/3D parsing, recursive validation via the Arlington model, and Rayon parallel optimization.
- **Governance & Diagnostic Infrastructure (Phase 11)**: Establishment of ELM conventions to prevent AI "memory evaporation" and the construction of a specification requirement extraction infrastructure using `pdf-spec-mcp`. [Complete]
- **Professional Integration (Phases 12-13)**: Thumbnail-driven page operations (Orchestrator), CAD-grade precision snap and measurement tools.
- **Semantic & Creative (Phases 14-15)**: Automatic generation and repair of tag structures by AI, and modeless context UX driven by selected objects.
- **Architectural Resilience (Phase 16)**: Construction of a Render Bridge to eliminate direct dependency on Vello.
- **Multibyte Text Rendering Precision (Phase 18)**: Resolution of overlapping and misalignment of Japanese text using specification-driven CID mapping. [Complete]
- **Legacy PDF Bridge & Zero-copy Architecture (Phase 19)**: Implementation of an isolated bridge for PDF 1.7 support (SJIS normalization, RC4/AES-128) and a high-performance `Bytes`-based internal data model. [Complete]

### 🗓 Future Outlook

- **3D Projection Rendering**: Real-time rendering support for 3D models using Vello.
- **App Packaging**: Automated builds of native installers for various operating systems.

Detailed implementation status is recorded in [ROADMAP.md](ROADMAP.md).

## Technical Stack

- **Language**: Rust (Edition 2024 / MSRV 1.94)
- **Rendering**: [Vello](https://github.com/linebender/vello) / WGPU
- **UI Framework**: [egui](https://github.com/emilk/egui) / eframe (0.33.1)
- **Compliance Standard**: ISO 32000-2:2020
- **Diagnostic Foundation**: `pdf-spec-mcp` (AI-exclusive specification query tool)

## Development Environment

- **Machine**: MacBook Air (Intel Core i5 / 16GB Memory)
- **OS**: macOS 15.7.4
- **AI Agent**: Antigravity (Gemini)
- **Dev Tools**: Cargo, Clippy, Rustfmt

## Rules for Development

We maintain the quality of AI-driven development through the following strict rules:

1. **HDD (Harness-Driven Development)**: Prepare a verification harness (test) before implementation.
2. **RR-15 (Reliable Rust 15)**: A set of 15 safety constraints.
3. **ELM (External Long-Term Memory)**: To prevent AI "memory evaporation," all thoughts and plans are persisted in real-time to `.agent/session/` within the project.
4. **Spec-First Diagnosis**: Eliminate inference and perform bug fixes based on the Scientific Method through specification verification using `pdf-spec-mcp`.

## File Structure

- `crates/`: Source code for the SDK core, rendering, UI, etc.
- `specs/`: Technical specifications treated as the "canon" of design.
- `samples/`: PDF samples for testing (Not included in the repository for copyright reasons).
- `scripts/`: Scripts for performing quality audits and compliance verification.

---

### Information for Developers & AI

Detailed protocols and workflows are centralized in the [.agent/](.agent/) directory.

- [AI Charter (GEMINI.md)](.agent/GEMINI.md): Behavioral principles for the AI.
- [Planning Protocol (PLANNING_PROTOCOL.md)](.agent/protocols/PLANNING_PROTOCOL.md): Rules for planning, synchronization, and management.
- [Quality Protocol (RELIABLE_RUST_15.md)](.agent/protocols/RELIABLE_RUST_15.md): 15 safety constraints.

---
© 2026 Ferruginous Project.
