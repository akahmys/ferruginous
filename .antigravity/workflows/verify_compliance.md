---
description: Automated verification workflow for RR-15 compliance.
---

# [Workflow] Verify Compliance

Audit compliance with [RR-15](../rules/hardening.md) from both the "skill" and "script" perspectives to maintain absolute quality.

## Procedure

// turbo
1. **AI Audit**: Follow the [verify_rr15](../skills/verify_rr15.md) skill to self-check the current codebase for any logical or structural violations.
2. **Automated Audit**: Execute the following and confirm that all items PASS mechanically.
// turbo
   - `./scripts/verify_compliance.sh` (Convention compliance)
// turbo
   - `cargo clippy --pedantic` (Zero warnings)
// turbo
   - `./scripts/msrv_check.sh` (Maintenance of compatibility)
3. **Artifact Recording**: Record audit results, discovered issues, and fix details in `walkthrough.md`.

## Completion Requirements
- Not a single warning or inconsistency remains.
- The AI's logical audit yields a judgment of "no room for improvement."
- If fixes are necessary, feature additions are suspended and fixes are prioritized.
