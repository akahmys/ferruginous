# Subagent Delegation Protocol

> [!IMPORTANT]
> **Scaling Through Delegation**: Use the `gemini` CLI to spawn isolated sub-thinkers for specialized, context-heavy, or verification-critical tasks. This preserves the primary agent's context for high-level orchestration.

---

## 1. Trigger Conditions (When to Delegate)

Delegate to a subagent when any of the following conditions are met:

- **Context Overflow**: Analyzing files or logs larger than 20KB that would otherwise pollute the primary context.
- **Verification (Cross-check)**: Requiring an independent second opinion on a critical fix or architectural decision.
- **Specification Lookup**: Searching for specific Clauses in ISO 32000-2 or Arlington Model definitions.
- **Boilerplate Generation**: Creating unit tests, documentation, or repeated code patterns based on a provided template.
- **Hypothesis Generation**: During "Fix Mode," generating multiple divergent hypotheses for a bug without committing to one.

## 2. Execution Procedure (How to Hire)

Use the `gemini` CLI via `run_command` with the following standards:

### A. Non-Interactive Mode
Always use the `-p` (prompt) flag for headless execution.
```bash
gemini -p "Analyze this log for memory leaks: $(cat logs/error.log)"
```

### B. Context Injection
Explicitly define the role and the target files.
- **Role**: "Act as an ISO 32000-2 Compliance Auditor."
- **Scope**: "Focus only on Clause 7.7.3.3 (CMap dictionaries)."
- **Input**: Provide file contents or clear paths if the subagent has MCP file access.

### C. Output Structuring
Request machine-readable or clearly formatted output to minimize parsing effort.
- "Format your findings as a JSON object."
- "Provide a bulleted list of 3 distinct hypotheses."

## 3. Integration of Results (Post-Delegation)

- **Validation**: Never take subagent output as "Truth." It must be verified by the primary agent through tests or inspection.
- **ELM Persistence**: Record significant subagent findings in `.antigravity/session/task.md` or `regression_log.md`.
- **Traceability**: If a subagent decision is used, note it in the `walkthrough.md` (e.g., "Hypothesis #2 provided by subagent-gemini was verified and implemented").

## 4. Forbidden Actions for Subagents

- **Direct File Mutation**: Subagents should *never* be allowed to write to production code directly. They are "Thinkers," not "Executors."
- **State Responsibility**: The primary agent remains the SSoT for the project state. Subagents are stateless tools.
