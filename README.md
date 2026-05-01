# Ferruginous: A Personal Quest for PDF 2.0 Fidelity

**Ferruginous** is an experimental but focused PDF processing platform built with Rust. It aims for **ISO 32000-2:2020** compliance through a deterministic, hardware-accelerated architecture, designed to handle complex PDF structures with high fidelity.

The project follows the **RR-15 (Reliable Rust-15)** safety protocol—a personal set of rules to ensure memory safety and predictable behavior in a mission-critical spirit.

**🚀 Status: Core Hardening Complete** — As of May 2026, the core modules have been refactored for better maintainability and safety.

---

## 🎯 Vision & Goals

*"Reliability over speed. Essence over prototyping."*

Ferruginous is a personal lab for exploring the limits of PDF technology. The "North Star" goals for this project are:

- **High Compliance**: Striving for the "Truth" of ISO 32000-2:2020 through thorough implementation.
- **Reliable Architecture**: Using Rust 2024 and the RR-15 protocol to minimize regressions and logic errors.
- **Better Typography**: A focus on CAD-grade visual quality via Vello/GPU, especially for complex CJK (Japanese) layouts.
- **AI-Friendly Design**: Keeping the codebase readable and verifiable for both human developers and AI assistants.

---

## 🤖 Building with AI (Antigravity)

This project is a collaborative effort between a human developer and an AI agent, **Antigravity**. 

### AI-Native Engineering
The architecture is designed to play well with autonomous AI tools:
- **Safety as Code**: Safety protocols (RR-15) are enforced and evolved through AI interaction.
- **Visual Verification**: Automated visual regression modules allow for "self-inspection" of rendering results.
- **MCP Native**: First-class support for the **Model Context Protocol**, enabling direct AI-to-PDF interaction.
- **Continuity Engine**: Design intents and stateful decisions are persisted to ensure seamless multi-session development.

---

## 🛡️ How it Works: The Ingestion Pipeline

Ferruginous doesn't just parse bytes; it tries to **ingest** and normalize them into a high-purity internal model through a multi-pass process.

### 1. The Normalization Process

- **Pass 0: Physical Guard (Normalization)**
    - **Recursive Decryption**: A stack-based walk of all objects to decrypt strings and streams.
    - **Security Handler Removal**: Stripping the `/Encrypt` dictionary from the trailer to ensure compatibility (Acrobat Error 135 prevention).
    - **Physical Repair**: Fixing broken XRef offsets and object numbers before they reach the `PdfArena`.
- **Pass 1: Arena Ingestion (Indexing)**
    - **Object Stream Expansion**: Unpacking compressed object streams.
    - **Generational Mapping**: Generating unique IDs and handles for every object.
    - **Deduplication**: Active identification of common resource objects.
- **Pass 2: Semantic Truth (Refinement)**
    - **Unicode-Native Pipeline**: Context-aware string re-encoding (`Byte` -> `UTF-8`) to eliminate mojibake.
    - **Color Sublimation**: Strict ICC profile application via **moxcms**.
    - **Structural Hardening**: Active remediation of logical structure tags for **ISO 14289-2 (PDF/UA-2)** compliance.

### 2. Memory & Safety: `PdfArena`

The core uses a generational arena to manage object lifetimes:
- **Handles over Pointers**: All object references are `Handle<Object>` (a `u32` index and a generation count). This prevents "use-after-free" and makes the structure AI-inspectable.
- **RR-15 Safety Invariants**: 
    - **Rule 2 vs 12**: Strict separation of input-driven errors (`Result`) from logical invariants (`assert!`).
    - **Rule 6 (Stack Safety)**: All object graph traversals must use an explicit stack and a hard-coded depth limit.
    - **Rule 10 (Determinism)**: Iteration and metadata generation is deterministic for bit-perfect output.
    - **Rule 11 (Transparency)**: Structured Enum errors only—no generic `String` or `anyhow` in core crates.

---

## 🔐 Security & Compliance

### Encryption Handling
Custom security handlers for PDF 1.4-2.0, focusing on Adobe fidelity:
- **AES-128 (Revision 4)**: High-fidelity MD5-based key derivation with correct `sAlT` handling.
- **AES-256 (Revision 5/6)**: SHA-256 based key derivation for PDF 1.7/2.0.
- **Advanced Recovery**: Robust UTF-16/PDFDocEncoding detection to eliminate mojibake in metadata.

---

## 🏛️ Project Structure

- **`ferruginous`**: 
    - 実験的なGUIアプリケーション。
    - **egui** と **wgpu** を統合し、120fpsのキャンバス描画とドキュメントの非同期ロードを実現。
- **`fepdf`**: 
    - 構造監査と修復のためのCLIツールキット。
    - サブコマンドベースの設計により、ストリームベースの修復や構造化されたJSON診断を提供。
- **`ferruginous-sdk`**: 
    - PDF操作のためのハイレベル・ライブラリ。
    - `PdfArena` を安全に操作するためのビルダーパターンや、セキュアな構造変更APIを提供。
- **`ferruginous-core`**: 
    - エンジンの心臓部。
    - 世代別 `PdfArena` による安全なオブジェクト管理、ISO 32000-2 準拠のパーサ、各種セキュリティハンドラ。
- **`ferruginous-render`**: 
    - GPU加速描画バックエンド。
    - **Vello** によるコンピュート・シェーダ描画、CJKグリフのキャッシュ管理、表示リストのシリアライズ。
- **`ferruginous-mcp`**: 
    - AIエージェントとの架け橋。
    - **Model Context Protocol** サーバを実装し、AIが直接PDFを診断・検証するためのツールを提供。

---

## 🛠️ CLI Toolkit (`fepdf`)

`fepdf` provides tools for batch processing and structural auditing.

### Subcommand Hierarchy
| Category | Command | Description |
| :--- | :--- | :--- |
| **Analyze** | `audit`, `info`, `text` | Document diagnostics and metadata inspection. |
| **Manipulate**| `merge`, `split`, `repair` | Structural modification and recovery. |
| **Produce** | `upgrade`, `sign`, `render` | PDF 2.0 conversion and GPU-accelerated output. |
| **Debug** | `dump`, `structure` | Low-level object and hierarchy visualization. |

### 🎯 Current Progress (CJK & Type 3)
- **Type 3 Support**: Improved metrics and `CharProcs` parsing for legacy Japanese PDFs.
- **Detection**: Dedicated Type 3 identification in audit reports.
- **Layout**: Focus on vertical writing and accurate glyph positioning for CJK.

---

## ⚙️ Development

- **Toolchain**: Rust 1.94+ / Edition 2024.
- **Verification**: Run `make verify` for the safety audit and regression suite.

---

## 📜 License

- **MIT License** / **Apache-2.0**
- Aiming for a technically sound ISO 32000-2:2020 baseline.
