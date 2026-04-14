---
description: Overall reorganization and cleanup of project conventions, skills, and workflows.
---

# [Workflow] Refactor Conventions

Eliminate redundancy in conventions as the project grows and rebuild the SSoT (Single Source of Truth).

## Procedure

// turbo
1. **Global Audit**: Scan `GEMINI.md`, `protocols/`, `skills/`, and `workflows/` to identify redundant descriptions or broken links.
// turbo
2. **Infrastructure Renewal**: Separate the roles of `GEMINI.md` (Charter) and `PLANNING_PROTOCOL.md` (Management), clarifying the division of responsibilities.
// turbo
3. **Unification of Descriptions**: Standardize each convention (e.g., `RELIABLE_RUST_15.md`) into the "Rule, Purpose, Criterion" format to eliminate ambiguity.
// turbo
4. **Extension of Automation**: Add `// turbo` annotations to all workflows and fix link paths to reflect the latest layout.
// turbo
5. **Organization**: Delete unnecessary temporary files and integrate misplaced specifications into the `specs/` directory.

// turbo
6. **Sync Verification**: Execute [sync_docs](sync_docs.md) to perform a final consistency check across all documents.

## Completion Requirements

- Maintenance of the hierarchical structure (Constitution -> Protocols -> Specifications)
- Equivalent quality assurance via `verify_compliance.sh`
- All links are functional
