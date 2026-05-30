# Subagent Delegation Protocol

> [!IMPORTANT]
> **Scaling Through Native Delegation**: Use Antigravity 2.0's native subagent orchestration (`define_subagent`, `invoke_subagent`, and `send_message`) to spawn isolated, specialized sub-thinkers and builders. This preserves the primary agent's context for high-level PM orchestration and enforces strict sandboxed execution.

---

## 1. Trigger Conditions (When to Delegate)

Delegate to a native subagent when any of the following conditions are met:

- **Context Preservation**: Analyzing complex files, extensive test output, or multi-crate logs that would otherwise pollute the primary PM context.
- **Role Isolation (Auditing)**: Requiring an independent compliance audit (ISO 32000-2 / RR-15) without contaminating the implementer's state.
- **Isolated Implementation**: Offloading complex feature additions or refactoring tasks to an Engineer subagent running in a separate workspace branch.
- **Specification Lookup**: Spawning a specialized researcher to lookup Arlington PDF models or ISO specifications using specialized MCP tools in parallel.
- **Hypothesis Generation**: Spawning multiple concurrent "self" subagents in separate conversation contexts to divergent-think bugs or edge-case behaviors.

---

## 2. Dynamic Definition & Invocation (How to Delegate)

Never use external CLI tools or run raw headless shell commands for thinking. Always use native Antigravity 2.0 orchestration:

### A. Define the Role (`define_subagent`)
Define a specialized subagent configuration. Tailor its system prompt and tool constraints to its unique role:

1.  **Read-Only / Research Mode**: Set `enable_write_tools = false` for auditors or researchers to prevent accidental codebase mutations.
2.  **Implementation Mode**: Set `enable_write_tools = true` to allow Engineer subagents to edit code and run test suites.
3.  **MCP Integration**: Enable `enable_mcp_tools = true` if the subagent requires search MCP servers (`cocoindex-code`, `pdf-spec`).

### B. Choose the Workspace Isolation Strategy (`invoke_subagent`)
Select the appropriate `Workspace` mode based on the safety and state requirements:

*   `branch`: **Highly Recommended for Engineering/Writing.** Clones the repository into an isolated git branch. The subagent can safely edit files, compile, and run tests. No changes are merged to the main workspace until the Auditor validates the branch.
*   `inherit`: **Recommended for Read-Only Research.** Inherits the parent's exact active workspace context. Excellent for scanning logs, lookups, and quick code auditing.
*   `share`: **Recommended for Coordinated Parallel Dev.** Shares the underlying directory (like git worktrees) for independent branch tracking.

### C. Message-Driven Communication (`send_message`)
Communication with active subagents must happen via `send_message`.
*   Avoid polling files or standard output. The IDE will automatically resume the parent agent's execution when a subagent sends a message.
*   Clearly state the **Handoff Interface Contract** in the initial prompt message to prevent semantic drift.

---

## 3. Integration of Results (Post-Delegation)

- **Validation Gate**: All code modifications produced in a subagent's `branch` must undergo a strict compliance audit by the primary agent and the Auditor role before being merged.
- **Walkthrough Traceability**: Document subagent contributions in `walkthrough.md` (e.g., "Feature X implemented by Engineer subagent `core-engineer-1` in an isolated workspace branch, verified via cargo test").
- **State Responsibility**: The Chief PM remains the sole Source of Truth (SSoT) for the project state and session progression. Subagents remain transient, stateless helpers.

---

## 4. Subagent Sandboxing Constraints

*   **No Uncontrolled Merges**: Subagents operating in a `branch` workspace cannot push directly to `main` or the parent's primary branch. All integrations must be requested back to the parent.
*   **Separation of Concerns**: An Auditor subagent must *never* have `enable_write_tools = true`. An Engineer subagent must not perform its own final gate compliance checks.
