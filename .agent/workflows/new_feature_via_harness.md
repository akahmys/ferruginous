---
description: Harness-driven development workflow for new features.
---

# [Workflow] Harness-Driven Development (New Feature via Harness)

Complete the implementation with a verification-led approach based on [HDD](../protocols/HDD_PROTOCOL.md) and [RR-15](../protocols/RELIABLE_RUST_15.md).

## Procedure

// turbo
1. **Preparation**: Create the target branch and complete [initialize_harness](initialize_harness.md).
// turbo
2. **Implementation**: Execute the `resolve_harness` skill to bring the tests to Green in compliance with RR-15.
// turbo
3. **Self-Audit**: Execute [verify_compliance](verify_compliance.md) to ensure static analysis and type safety.
// turbo
4. **Document Reflection**: Execute [sync_docs](sync_docs.md) to synchronize the ROADMAP and task with the latest actual state.

## Completion Requirements

- All tests pass (unit, integration, and negative tests).
- Zero Clippy (Pedantic) warnings.
- Link descriptions to ISO Clauses.
