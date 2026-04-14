# Strategic Planning Protocol

> [!IMPORTANT]
> **AI Survival Strategy**: Execute the "3.1. Session Start Procedure" of this protocol immediately at the start of every session. Externalize every thought process to ELM instantly.

---

## 1. Single Source of Truth (SSoT)

- **Rule**: Maintain project consistency by strictly categorizing assets into their respective SSoT roles.
- **Purpose**: Prevent information fragmentation and ensure that all development decisions are based on the latest canonical source.
- **Criterion**:
    - **Constitution**: `.agent/GEMINI.md` (Fundamental principles)
    - **Governance**: `.agent/protocols/` (Quality & process constraints)
    - **Specs**: `specs/` (Internal design & requirements)
    - **External Standard**: ISO 32000-2 (via `pdf-spec-mcp`)
    - **Memory (ELM)**: `.agent/session/` (Real-time thought/plan persistence)
    - **History**: `ROADMAP.md` (Approved milestones)

## 2. ELM Lifecycle (Immediate Persistence)

- **Rule**: Physically persist every plan, task, and result to the `.agent/session/` directory in the same turn they are created.
- **Purpose**: Overcome "Session Amnesia" by ensuring that thoughts exist in the project filesystem, not just in AI context.
- **Criterion**: All active `task.md` and `implementation_plan.md` states must match the local filesystem files before the session turn ends.

## 3. Execution Modes

- **Rule**: Every task must be categorized into either **Build Mode** or **Fix Mode**, and the corresponding protocol must be declared.
- **Purpose**: Apply the appropriate verification rigor (HDD for new features, Diagnosis-first for fixes).
- **Criterion**:
    - **Build Mode**: Applies to new features. Use [HDD_PROTOCOL](HDD_PROTOCOL.md).
    - **Fix Mode**: Applies to bugs/regressions. Use [FIX_PROTOCOL](FIX_PROTOCOL.md).

## 4. Session Management

### 4.1. Session Start Procedure [MANDATORY]
- **Rule**: Upon startup, sync with ELM sequentially: Constitution → Memory (Handoff/Regression) → Task/Plan Sync → Mode Declaration.
- **Purpose**: Re-establish context and historical awareness before making any modifications.
- **Criterion**: The first response of a session must demonstrate awareness of the last completed task and the current goal.

### 4.2. WAL (Write-Ahead Log) Operation
- **Rule**: Declare intent in `task.md` *before* execution, and record facts (test results/diffs) *after* execution.
- **Purpose**: Create a traceable audit trail of "Why" and "What" for future session diagnosis.
- **Criterion**: Every code-modifying turn must have a corresponding entry in the session task list.

## 5. Portability & Integrity

- **Rule**: Enforce relative paths and strictly anonymize all logs/comments.
- **Purpose**: Ensure the repository can be cloned and managed on any system without environment-specific failures.
- **Criterion**: Zero absolute paths (`/Users/...`) in project documentation or ELM files.

## 6. Version Control Workflow

- **Rule**: Balance the stability of `main` with the flexibility of development branches using a strict branch-per-milestone lifecycle.
- **Purpose**: Maintain `main` as the stable "SSoT of Stability" while allowing bold, diagnostic changes in development branches.
- **Criterion**:
    - **main Branch**: Must always pass `verify_compliance.sh`. Only merged after user approval.
    - **Development Branches**:
        - **Build Mode**: Use `feat/phase-[XX]-[description]`.
        - **Fix Mode**: Use `fix/[issue-description]`.
    - **Lifecycle**:
        1. Create a new branch at the start of a phase.
        2. Perform diagnosis, implementation, and verification on the branch.
        3. Pass the completion gate (all tests pass) and report with `walkthrough.md`.
        4. After user approval, merge into `main` and close the branch.
