# Strategic Planning Protocol

> [!IMPORTANT]
> **AI Survival Strategy**: Execute the Session Start Procedure immediately at the start of every session. Externalize every thought process to ELM (External Long-Term Memory) instantly.

---

## 1. Single Source of Truth (SSoT)
- **Rule**: Strictly categorize all project assets into their respective SSoT roles (Constitution, Governance, Specs, Standards).
- **Purpose**: Prevent information fragmentation and ensure all development decisions are based on the latest canonical source.
- **Compliance Criterion**: All architectural and process-based decisions must reference the designated SSoT file.

## 2. ELM Lifecycle (Immediate Persistence)
- **Rule**: Physically persist all plans, tasks, and results to the `.agent/session/` directory in the same turn they are created.
- **Purpose**: Overcome "Session Amnesia" by ensuring thoughts exist in the project filesystem, not just in volatile AI context.
- **Compliance Criterion**: Local filesystem states for `task.md` and `implementation_plan.md` must be synchronized before each turn ends.

## 3. Mandatory Session Bootstrapping
- **Rule**: Every new session must begin with a sequential sync: Constitution → Memory → Task/Plan Sync → Mode Declaration.
- **Purpose**: Re-establish professional context and historical awareness before making any system modifications.
- **Compliance Criterion**: The first turn of any session must demonstrate explicit awareness of the current goal and last completed task.

## 4. WAL (Write-Ahead Log) Operation
- **Rule**: Declare intent in `task.md` *before* execution, and record factual results (tests/diffs) *after* execution.
- **Purpose**: Create a traceable audit trail of "Why" and "What" for future diagnosis and recovery.
- **Compliance Criterion**: Every code-modifying turn must have a corresponding, updated entry in the session task list.

## 5. Branch-per-Milestone Lifecycle
- **Rule**: Isolation of development work in branches following the [GITHUB_MERGE_PROTOCOL](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/GITHUB_MERGE_PROTOCOL.md) and merging to `main` only after 100% verification and user approval.
- **Purpose**: Maintain `main` as a continuously stable and compliant foundation.
- **Compliance Criterion**: `main` must always pass `verify_compliance.sh` and contain only verified, approved features.
