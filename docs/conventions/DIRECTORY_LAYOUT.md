# Ferruginous Workspace Directory Layout

(ISO 32000-2 Compliance)

This document formalizes the directory structure of the Ferruginous repository to ensure long-term "Hardened" integrity and clarity.

## Layout Overview

| Directory | Semantic Purpose | Responsibility |
| :--- | :--- | :--- |
| `assets/` | **Bundled Resources** | Project-specific fonts and unique assets required for engine fallback. |
| `crates/` | **Source Code** | The core Rust modular architecture of the PDF engine. |
| `docs/` | **Knowledge Base** | High-level specifications, retrospectives, and engineering conventions. |
| `examples/` | **API Reference** | Real-world usage examples demonstrating how to integrate the library. |
| `external/` | **Reference Data** | Consolidated third-party data (Adobe CMaps, Arlington PDF model). |
| `samples/` | **Reference Samples** | A curated collection of PDF documents for visual and manual verification. |
| `scratch/` | **Dev Playground** | Transient folder for developer experiments. (Ignored by VCS). |
| `scripts/` | **Automation** | Categorized utility scripts for `audit`, `dev`, and `test`. |
| `tests/` | **Regression Suite** | Integration tests and binary snapshots (fixtures). |
| `verification_results/` | **Logs & Output** | Destination for audit logs and rendering outputs. (Ignored by VCS). |

## Governance Rules

1.  **No Redundancy**: Do not copy files from `external/` to `assets/`. Point the engine directly to the unified `external/` paths.
2.  **Script Placement**: Always place new automation in the appropriate `scripts/` subdirectory (`audit`, `dev`, or `test`).
3.  **Clean Root**: Keep the project root clean. Only core project metadata (`README`, `ROADMAP`, `VISION`, `LICENSE`) and workspace Cargo files should reside here.
