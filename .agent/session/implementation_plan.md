# Protocol & Workflow Consistency Refinement Plan

This plan addresses contradictions and pathing inconsistencies identified during the scrutiny of the newly designed protocols. The goal is to ensure that the "Externalized Memory" system works smoothly across all protocols and automation workflows.

## User Review Required

> [!IMPORTANT]
> **Implementation Plan Location**: Currently, my implementation plans are artifacts in the hidden "brain" directory. I propose to mirror them to `.agent/session/implementation_plan.md` (as a standard Markdown file) so that the next AI session can read the *entire* plan and its reasoning without relying on the system-provided context.

## Proposed Changes

### [Protocols Refinement]

#### [MODIFY] [PLANNING_PROTOCOL.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/PLANNING_PROTOCOL.md)
- Update SSoT table to specify `.agent/session/implementation_plan.md`.
- Ensure all references to `walkthrough.md` have the `.agent/session/` prefix.
- Clarify script references to `scripts/verify_compliance.sh`.

#### [MODIFY] [FIX_PROTOCOL.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/FIX_PROTOCOL.md)
- Add `.agent/session/` prefix to all file references (`task.md`, `walkthrough.md`, `regression_log.md`).
- Ensure consistent numbering of the 9-step cycle with the `task.md` WAL format.

#### [MODIFY] [HDD_PROTOCOL.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/HDD_PROTOCOL.md)
- Standardize `.agent/session/` prefixes.
- Link to `scripts/verify_compliance.sh` instead of generic names.

#### [MODIFY] [COMPLIANCE_STRATEGY.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/COMPLIANCE_STRATEGY.md)
- Remove references to `safety_vitals.sh` (deprecated).
- Point exclusively to `verify_compliance.sh`.

### [Workflows Refinement]

#### [MODIFY] [sync_docs.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/workflows/sync_docs.md)
- Update all file paths to point to their new locations under `.agent/session/`.

#### [NEW] [fix_via_diagnosis.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/workflows/fix_via_diagnosis.md)
- Create a specific workflow that implements the [FIX_PROTOCOL](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/FIX_PROTOCOL.md) steps as a // turbo automated process.

### [Scripts Consolidation]

#### [MODIFY] [safety_vitals.sh](file:///Users/jun/Documents/Project/Ferruginous/scripts/safety_vitals.sh)
- Turn into a thin wrapper for `verify_compliance.sh` or mark as deprecated.

## Verification Plan

### Automated Tests
- Run `ls -R .agent/session` to ensure no accidental double files (e.g., `task.md` vs `.agent/session/task.md`).
- Run `scripts/verify_compliance.sh` to ensure it still passes after path changes.

### Manual Verification
- I will perform a search for "task.md" across the `.agent/` directory to ensure no local-path (pathless) strings remain.
