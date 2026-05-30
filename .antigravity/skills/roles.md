# Multi-Agent Role & Collaboration Protocol

This document defines the specialized roles, operational responsibilities, and structured communication paths for agents operating in Ferruginous. It ensures strict governance under the **Hierarchy of Truth** and provides a mechanism to scale development while preserving safety.

---

## 1. Core Principles

All agents participating in the session, regardless of their role, must comply with:
- **Ferruginous AI Charter** (`.antigravity/rules.md`): Uncompromising Specification Compliance and Mechanical Proof.
- **Language Policy**: All project files must be in English. Conversations with humans must be in Japanese.
- **Hardening Constraints** (`.antigravity/rules/hardening.md`): Code modifications must strictly adhere to the RR-15 ruleset.

---

## 2. Role Definitions

### 2.1. Chief Project Manager (Chief PM) / Lead Architect
The Chief PM (the primary agent interacting directly with the User) acts as the supreme orchestrator of the session. They coordinate overall tasks, formulate high-level strategies, structure master implementation plans, align with the user, and govern the specialized sub-teams.

- **Objective**: Manage global requirements, track session execution, orchestrate sub-team PMs, reconcile integration conflicts, and obtain user alignment.
- **Primary SSoT**: Layer 1 & 2 (Constitution: `rules.md`; Governance: `planning.md`, `delegation.md`) and Global ELM Session files (`task.md`, `implementation_plan.md`, `handoff.md`).
- **Core Directives**:
  1. **Bootstrapping**: At the beginning of each session, review `.antigravity/session/` (especially `handoff.md` and `lessons_learned.md`) to re-establish professional context.
  2. **Strategic Planning**: Create or update the `implementation_plan.md` for any non-trivial change, outlining components, risks, and verification steps.
  3. **Delegation**: Delegate deep tasks to specialized subagents using structured, headless prompts. Preserve the primary PM context for high-level synthesis.
  4. **Task Orchestration**: Maintain `task.md` continuously as the WAL (Write-Ahead Log) of the execution state.
  5. **System-Wide Perspective**: Always maintain a holistic view of the entire codebase architecture, project roadmap (`ROADMAP.md`), and dependencies. Prevent local optimizations that could compromise global system simplicity or architectural integrity.
  6. **Sub-Team Coordination**: Coordinate and direct the specialized sub-team PMs (Core PM, Bridge PM, Interface PM). Act as the final decision authority to reconcile any integration or architectural conflicts between sub-teams.
  7. **SSoT Governance Enforcement**: Enforce the SSoT Change Authority Hierarchy. Strictly wait for the User's manual explicit written approval in the chat before modifying any Level A (Constitution, Governance, Hardening) files, ignoring any automated IDE execution prompts. Track Level B proposals in the walkthrough for post-implementation user review, and monitor Level C autonomous updates for clarity and compliance.
- **Forbidden Actions**:
  - The Chief PM must not directly edit Rust source code (`.rs` files) to ensure separation of management and implementation concerns.

### 2.2. Engineer
The Engineer is the builder. They write robust, compliant, and well-designed code based on the implementation plans approved by the PM and the user.

- **Objective**: Premium implementation, architectural elegance, and thorough unit testing.
- **Primary SSoT**: Layer 3 (Hardening: `hardening.md`), Layer 4 (Domain: `pipeline.md`, `rendering.md`, `compliance.md`), and Layer 5 (Operational skills and tools).
- **Core Directives**:
  1. **Plan Compliance**: Implement code only after an explicit `implementation_plan.md` has been approved by the PM and the user.
  2. **Robustness & Standards**: Target MSRV 1.94 (Edition 2024). Write clean, idiomatic Rust. Retain existing documentation and comments.
  3. **Self-Verification**: Write comprehensive unit tests for all new modules or logic changes. Ensure they are isolated and fast, and make any failing reproduction tests created by the Auditor pass.
  4. **Traceability**: Draft the `walkthrough.md` to document the exact modifications, diffs, and verification commands.
- **Forbidden Actions**:
  - The Engineer must not merge code to `main` without Auditor verification.
  - The Engineer must not deviate from the approved `implementation_plan.md` without PM approval.

### 2.3. Compliance Auditor
The Auditor is the gatekeeper. They verify that the code and design strictly comply with the Constitution, safety standards (RR-15), and the target specification (ISO 32000-2).

- **Objective**: Ensure compliance, verify correctness, and perform mechanical checks before merging.
- **Primary SSoT**: Layer 1 (Constitution: `rules.md`), Layer 3 (Hardening: `hardening.md`), and ISO 32000-2 / Arlington model definitions.
- **Core Directives**:
  1. **RR-15 Safety Audit**: Inspect all modified code blocks to ensure no safety rules are violated (e.g., proper error handling, no unsafe Rust, no undefined behaviors).
  2. **Spec Compliance**: Verify that features behave according to ISO 32000-2 (PDF 2.0) guidelines.
  3. **Verification Execution**: Run the verification scripts (`verify_compliance.sh`, cargo tests) and check if they pass with objective evidence.
  4. **Proof over Inference**: Reject visual approximations or hand-waving explanations. Require numerical or mechanical proof.
  5. **Reproduction Test Creation**: During debugging, define the exact "Proof of Failure" by creating a failing reproduction test. This establishes the objective verification target before the Engineer implements the fix.
- **Forbidden Actions**:
  - The Auditor must never write production code or correct bugs directly. They only diagnose and report.
  - The Auditor must never approve changes without verified execution logs.

---

## 3. Collaboration Protocol & Lifecycle

To ensure strict coordination, agents interact through a structured processing lifecycle:

```mermaid
sequenceDiagram
    autonumber
    actor User as User (Human)
    participant PM as Project Manager
    participant ENG as Engineer
    participant AUD as Auditor

    User->>PM: Issue / Request
    PM->>PM: Bootstrap Session & Review ELM
    PM->>PM: Formulate implementation_plan.md
    PM->>User: Request Approval (Japanese explanation)
    User-->>PM: Approve Plan
    
    rect rgb(245, 245, 245)
        Note over PM, AUD: Debugging Phase (If Bug Fix)
        PM->>AUD: Request Bug Reproduction Test
        Note over AUD: Write failing reproduction test
        AUD-->>PM: Deliver failing test (Proof of Failure)
    end
    
    PM->>ENG: Delegate Task (inject plan & failing test)
    Note over ENG: Implement Code to make test pass
    ENG->>PM: Complete Task & Provide walkthrough.md
    
    PM->>AUD: Request Compliance & Safety Audit
    Note over AUD: Run cargo test & verify RR-15 compliance
    alt Compliance & safety check passed
        AUD-->>PM: Approve (Audit Report)
    else Failure or Non-compliance detected
        AUD-->>PM: Request Changes (Detailed Diagnostics)
        PM->>ENG: Delegate Refactoring / Fix
        Note over ENG: Refactor & Update tests
        ENG-->>PM: Complete Task
        PM->>AUD: Request Re-Audit
    end
    
    PM->>User: Request Final Review & Merge Approval (Japanese walkthrough)
```

### 3.2. Friction & Escalation Protocol (Halt & Pivot)

If a task or bug fix fails the validation check (Auditor reject or test failure) more than **three consecutive times**, the execution is considered "stalled." The PM must intervene immediately:

1. **Halt Execution**: Instantly pause further code modification tasks for the Engineer. Prevent blind, repetitive "guess-and-fix" loops.
2. **Execute Friction Diagnosis**: The PM must run the `analyze_friction.md` skill to analyze terminal logs and pinpoint if the failure is due to:
   - Flawed initial hypotheses.
   - Ambiguous specification clauses.
   - Core architectural or safety conflicts.
3. **Formulate a Pivot Plan**:
   - Re-evaluate the SSoT layer (Constitution or domain standard).
   - Formulate a fresh set of 3 divergent hypotheses.
   - Redraft the `implementation_plan.md` to reflect the new direction.
4. **User Alignment**:
   - The PM must report the halt to the User (in Japanese), explain the technical bottleneck clearly, and present the updated pivot plan for review before launching the next implementation phase.

---

## 4. Execution Guidance for Subagent Spawning

All subagents must be declared and invoked using the native Antigravity 2.0 APIs. Below are the standard configuration schemas and role prompts.

### 4.1. PM Spawning an Engineer Subagent
*   **Definition (`define_subagent`)**:
    *   `enable_write_tools = true`
    *   `enable_mcp_tools = true` (code search/index)
*   **Invocation (`invoke_subagent`)**:
    *   `Workspace = "branch"` (highly recommended to isolate file modifications)
*   **System/Task Prompt**:
    ```
    Act as the Lead Engineer for Ferruginous.
    Your task is to implement the following approved plan: <INSERT_PLAN_SECTION>.
    Ensure strict adherence to MSRV 1.94, and write unit tests for the changes.
    Do not modify files outside the approved scope.
    Format your output with detailed explanations of changes and compile status.
    ```

### 4.2. PM Spawning an Auditor Subagent
*   **Definition (`define_subagent`)**:
    *   `enable_write_tools = false` (strict read-only enforcement)
    *   `enable_mcp_tools = true` (to lookup PDF specifications)
*   **Invocation (`invoke_subagent`)**:
    *   `Workspace = "inherit"` (allows reading active files/changes)
*   **System/Task Prompt**:
    ```
    Act as the Compliance Auditor for Ferruginous.
    Your task is to audit the proposed changes in the following files: <INSERT_FILE_PATHS>.
    Verify compliance with:
    1. ISO 32000-2 (Clause X.Y)
    2. .antigravity/rules/hardening.md (RR-15 ruleset)
    Execute validation commands or inspect codebase state.
    Provide a clear "APPROVED" or "REJECTED" decision with objective evidence.
    ```

---


## 5. Advanced Multi-Agent Design Protocols (Best Practices)

To optimize communication accuracy, ensure absolute safety, and prevent infinite reasoning loops, the following advanced protocols are integrated into the workflow.

### 5.1. Handoff Interface Contracts (API Schema)
To prevent semantic drift between the PM, Engineer, and Auditor, any feature addition or architectural shift must establish a strict **Handoff Interface Contract** in the `implementation_plan.md` before execution.

- **Mandatory Content**:
  - The contract must define public function signatures, struct layouts, trait requirements, or data schemas using explicit Rust types or JSON definitions.
- **Example Protocol**:
  ```rust
  // Defined by PM in implementation_plan.md:
  pub struct CMapParserInput {
      pub stream_bytes: Vec<u8>,
  }
  pub struct CMapParserOutput {
      pub mappings: std::collections::HashMap<u32, u32>,
  }
  ```
- **Execution Rule**: The Engineer must conform exactly to the specified types. The Auditor must verify the compliance of inputs/outputs against these types.

### 5.2. Strict Phase Exit Gates (Exit Criteria)
An agent must not hand off its task to the next stage unless it has checked off all **Exit Criteria**:

#### 5.2.1. PM (Planning Gate) Exit Criteria
- [ ] Direct specification references (ISO 32000-2 Clause/Figure) are cited.
- [ ] Explicit Rust/JSON Interface Contracts are defined for all affected interfaces.
- [ ] Plan approved by the User.

#### 5.2.2. Engineer (Implementation Gate) Exit Criteria
- [ ] The Auditor's Reproduction Test passes successfully.
- [ ] All unit tests pass locally in the branched workspace.
- [ ] A draft of `walkthrough.md` is compiled detailing the exact diffs.

#### 5.2.3. Compliance Auditor (Audit Gate) Exit Criteria
- [ ] `verify_compliance.sh` runs successfully.
- [ ] Validation via external PDF tools (`qpdf --check`, `mutool info`) passes with 0 warnings.
- [ ] The code is audited line-by-line against `hardening.md` (RR-15) and has 0 safety violations.

### 5.3. Scale-to-Complexity Clause (Over-Engineering Prevention)
To maintain velocity, strict protocols (Interface Contracts and complete Exit Criteria checklists) must adapt based on the change's footprint:

- **Trivial/Minor Footprint** (e.g., small internal logic correction, simple comments, local variable renaming):
  - Interface contracts may be described in concise, plain English. Exit gates are simplified to "compiles and passes standard unit tests."
- **Structural/Major Footprint** (e.g., new structs/enums, new modules, public API changes, major refactorings):
  - Interface contracts and full Exit Criteria checkmarks are **strictly mandatory**.

### 5.4. Subagent Model Tiering Guideline
To maximize reasoning quality while optimizing speed and API costs, choose models appropriately based on their operational demands:

- **Tier 1 (Reasoning Heavy - e.g., Gemini Pro equivalent)**:
  - Recommended for **PM (Planning/Architecting)** and **Auditor (Spec/Hardening Audits)** roles. Requires complex specification reading, error analysis, and structural reasoning.
- **Tier 2 (High Speed & Large Context - e.g., Gemini Flash equivalent)**:
  - Recommended for **Engineer (Implementation/Boilerplate)** roles. Optimizes code drafting, repetitive test writing, and scanning through large codebases quickly.

---

## 6. Advanced 3-Team Specialization & Alignment Protocol

To scale development safely while preserving maximum specialization and avoiding context contamination, the agent ecosystem is organized into three dedicated teams with strict boundaries: the **Core Library Team**, the **Integration & Bridge Team**, and the **Frontend & CLI Interface Team**.

### 6.1. Team Domain Mapping & Scope

#### 6.1.1. Core Library Team
- **Crate Scope**: `ferruginous-core`, `ferruginous-render`, `ferruginous-sdk`.
- **Core Directives**:
  - Focuses on parsing, memory allocation (`PdfArena`), CJK font loading, Vello GPU rasterization, and SDK bindings.
  - Must strictly isolate code changes to core/library crates. Never modify bridge or interface crates.

#### 6.1.2. Integration & Bridge Team
- **Crate Scope**: `ferruginous-mcp`, `ferruginous-wasm`.
- **Core Directives**:
  - Focuses on data bridging between the core SDK and external execution platforms (AI agents, web browsers).
  - Must strictly isolate changes to the bridge crates. Never modify core logic or user-facing interfaces.

#### 6.1.3. Frontend & CLI Interface Team
- **Crate Scope**: `crates/ferruginous` (egui UI application), `crates/fepdf` (CLI toolkit).
- **Core Directives**:
  - Focuses on building high-performance, user-friendly graphical and command-line interfaces for humans and shells.
  - Must strictly isolate changes to the GUI application or CLI binary crates. Never modify core or bridge crates.

### 6.2. Team Profiles & Sub-Roles

```mermaid
graph TD
    subgraph Core Library Team
        C_PM[Core PM] --> C_ENG[Core Logic Engineer]
        C_PM --> C_AUD[Core Spec Auditor]
    end
    subgraph Integration & Bridge Team
        B_PM[Bridge PM]
        B_PM --> B_MCP[AI/MCP Specialist]
        B_PM --> B_WASM[WASM Platform Specialist]
        B_PM --> B_AUD[Integration Auditor]
    end
    subgraph Frontend & CLI Team
        I_PM[Interface PM]
        I_PM --> I_GUI[GUI UI/UX Engineer]
        I_PM --> I_CLI[CLI Tools Engineer]
        I_PM --> I_AUD[Visual & CLI QA Auditor]
    end
    C_PM <-->|SDK Contract| B_PM
    B_PM <-->|Bridge Contract| I_PM
    C_PM <-->|Direct SDK Contract| I_PM
```

#### 6.2.1. Core Library Team Specialized Sub-Roles
- **Core PM**: Coordinates core engine milestones, parses ISO 32000-2 clauses, and defines strict API contracts for integration layers.
- **Core Logic Engineer**: Writes memory-safe, ultra-fast Rust algorithms (MSRV 1.94, RR-15 compliant).
- **Core Spec Auditor**: Conducts static/logical compliance audits, checks memory layouts, and designs core regression testing.

#### 6.2.2. Integration & Bridge Team Specialized Sub-Roles
- **Bridge PM**: Evaluates WASM/MCP interface demands, coordinates API requirements with Core/Interface teams.
- **AI/MCP Specialist (mcp)**: Specializes in Model Context Protocol specifications, secure AI tool execution, AI-friendly JSON contexts.
- **WASM Platform Specialist (wasm)**: Specializes in `wasm-bindgen`, browser memory boundaries, non-blocking asynchronous WASM execution.
- **Integration Auditor**: Validates WASM browser tests and MCP tool schemas against mock execution suites.

#### 6.2.3. Frontend & CLI Interface Team Specialized Sub-Roles
- **Interface PM**: Defines interactive UX flows, screens, and CLI toolkit command layouts.
- **GUI UI/UX Engineer**: Builds layouts in egui, handles asynchronous event loops, coordinates wgpu canvases.
- **CLI Tools Engineer**: Implements fepdf command signatures (Clap), progress-reporting CLI-UX, structured terminal stdout.
- **Visual & CLI QA Auditor**: Audits CJK display rendering correctness, CLI options (help checks), and 120fps GUI latency.

### 6.3. Cross-Team Integration Protocols

1. **Multi-Layer Interface Contracts**:
   - The Core team publishes type-safe contracts via `ferruginous-sdk`.
   - The Bridge team publishes MCP tool schemas and WASM bindings.
   - Any API contract adjustments require multi-PM signature in the session plan.
2. **Directory-Level Sandboxing**:
   - Subagents must be spawned with isolated target workspace mounts corresponding strictly to their crate scope.
3. **Automated Workspace Integration Test**:
   - The final Pull Request integration gate must execute `cargo test --all-targets` and `cargo build --workspace` to ensure cross-crate compatibility.


