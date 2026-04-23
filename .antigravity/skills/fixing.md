# Fix Protocol

> [!IMPORTANT]
> **Nature of a Fix**: "Code changes" and "Solutions" are distinct. Strictly adhere to the Scientific Method and eliminate side effects from guesswork.

---

## 1. Scientific Diagnosis
- **Rule**: Prior to any code changes, perform a diagnosis to narrow down the cause to a single hypothesis.
- **Purpose**: Eradicate side effects and "band-aid" fixes by identifying the root cause.
- **Compliance Criterion**: `task.md` must document the phenomena, hypotheses, and diagnostic results (including structural/lookahead consistency for syntax errors) before implementation begins.

## 2. Specification Alignment
- **Rule**: Every fix must be verified against ISO 32000-2 requirements via `pdf-spec-mcp`.
- **Purpose**: Ensure that the "fix" actually aligns with the international standard, not just a specific file's quirks.
- **Compliance Criterion**: The fix must cite the relevant ISO Clause and proof of compliance.

## 3. Minimal Intervention
- **Rule**: Modify only the minimum necessary logic required to address the diagnosed cause.
- **Purpose**: Characterize the fix's scope and minimize the surface area for new regressions.
- **Compliance Criterion**: The resulting diff must be focused solely on the bug's root cause.

## 4. Regression Verification
- **Rule**: A fix is not complete until a reproduction test passes and all existing tests in the workspace pass.
- **Purpose**: Prove the effectiveness of the fix and the stability of the entire system.
- **Compliance Criterion**: `cargo test --all-workspace` must return success, and `walkthrough.md` must link to the passing reproduction test.

## 5. Knowledge Conversion
- **Rule**: Record lessons learned and diagnostic patterns in `regression_log.md`.
- **Purpose**: Promote institutional memory and prevent AI contexts from forgetting common pitfalls.
- **Compliance Criterion**: `regression_log.md` must be updated with the terminal state of the fix.
