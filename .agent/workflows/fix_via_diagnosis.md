---
description: Automated workflow for bug fixing based on FIX_PROTOCOL.
---

# [Workflow] Fix via Diagnosis

Fixing is not development. Identify the root cause of the bug and resolve it with minimal changes.

## Procedure

1. **Session Initialization**: Follow the startup instructions in [.agent/GEMINI.md](../../.agent/GEMINI.md), loading handoff and regression_log.
// turbo
2. **Describe the Phenomenon**: Observe the phenomenon and accurately record the current status in [.agent/session/task.md](../session/task.md).
// turbo
3. **Verify Past History**: Read [.agent/session/regression_log.md](../session/regression_log.md) and list past failure patterns.
// turbo
4. **Enumerate Hypotheses**: List **at least three** hypotheses in task.md explaining why the issue is occurring.
// turbo
5. **Design & Execute Diagnosis**: Verify hypotheses without changing production code. Add diagnostic logs or create new tests.
// turbo
6. **Implement Minimal Change**: Based on the most probable hypothesis, change only one location in the production code.
// turbo
7. **Verify Effect**: Confirm that the symptoms are resolved via tests or visual inspection (UI only).
// turbo
8. **Regression Check**: Run `scripts/verify_compliance.sh` and all automated tests to prove that no new breakage has occurred.
// turbo
9. **Record & Finish**: Record the results in task.md, regression_log.md, and handoff.md, then end the session.

## Completion Requirements

- A bug reproduction test has been added to the test suite and is PASSING.
- All existing tests are PASSING.
- Empirical evidence is recorded in [.agent/session/walkthrough.md](../session/walkthrough.md).
- Final approval has been obtained from the user.
