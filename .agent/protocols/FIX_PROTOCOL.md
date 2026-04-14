# Fix Protocol

> [!IMPORTANT]
> **Nature of a Fix**: "Code changes" and "Solutions" are distinct.
> Strictly adhere to the Scientific Method (Diagnosis → Hypothesis Verification → Minimal Change) and eradicate side effects caused by guesswork.

## 1. Operating Mode: Fix Mode

Applied when correcting inconsistencies in existing code, regressions, or specification violations. Success is defined as "the intended fix is implemented and everything else remains unbroken."

## 2. Epistemic Humility

- **Principle**: The AI must always doubt the confidence level of its own hypotheses.
- **Requirement**: A fix must not be considered complete without objective evidence (passing tests, log outputs, or Clauses from the specification).

## 3. The Fix Cycle

Execute the following 9 steps without omission.

1.  **Describe the Phenomenon**: Exactly what is happening must be persisted to ELM in `task.md`.
2.  **Reference History**: Read `regression_log.md` to identify similar past incidents or failures.
3.  **Multifaceted Hypotheses**: List multiple candidate causes and do not fixate on one.
4.  **Design the Diagnosis**: Design a method to narrow down hypotheses **without changing any** production (live) logic.
    - **Spec-First**: Extract expected values (shall/must) from the specification using `pdf-spec-mcp`.
5.  **Execute the Diagnosis**: Use log outputs or diagnostic scripts to narrow down the hypotheses to one.
6.  **Verify Hypothesis against Facts**: Clarify the difference between expected values (specification) and measured values (diagnostic results).
7.  **Minimal Change**: Fix **only the minimum necessary point** identified as the cause.
8.  **Verify Effect and Regression**: Confirm the disappearance of the symptoms and prove via all tests that there is no impact on other areas.
9.  **Convert to Learning**: Record the result in `regression_log.md` as a lesson for future AI self.

## 4. Diagnostic Toolkit

- **`pdf-spec-mcp`**: Extraction of ISO 32000 conditions, table definitions, and inheritance rules.
- **Diagnostic Probes**: Output internal matrices (CTM, FontMatrix, etc.) using `eprintln!`.
- **Verification Harnesses**: Disposable binaries (e.g., `diag_layout`) that extract and verify specific behaviors.

## 5. Completion Gate

A fix can only be declared [Complete] once all of the following [MUST] items are met.

- **Passing Reproduction Test**: A test that reproduces the fixed bug must PASS after the fix.
- **Passing All Existing Tests**: Prove there are no regressions by passing all tests via `cargo test`.
- **ELM Persistence of Evidence**: Logs and `walkthrough.md` must be synchronized to the project directory.
- **User Approval**: User approval is required for any updates to the ROADMAP.

---
> [!CAUTION]
> **Prohibited Items**: Changing logic without diagnosis and reporting completion based "solely on visual inspection."
