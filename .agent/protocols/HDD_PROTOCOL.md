# Harness-Driven Development (HDD) Protocol

> [!IMPORTANT]
> **Build Mode**: Applied during new feature development.
> Eliminate inference and prioritize "Mechanical Proof." Construct a verification environment (Harness) prior to implementation to dynamically prove consistency with ISO 32000-2.

## 1. Spec-First

- **Rule**: Prior to implementation, always use `pdf-spec-mcp` to extract the relevant ISO 32000-2 Clause and requirements (shall/must).
- **Purpose**: Eliminate developer subjectivity and ensure "Full Compliance" with international standards at the design stage.
- **Criterion**: The `implementation_plan.md` must cite Clause numbers and extracted requirements.

## 2. Harness-First

- **Rule**: Prior to logic implementation, always write a "test that fails as expected" or a "probe to verify output."
- **Purpose**: Confirm that the test itself correctly reflects the requirements and prevent debugging from going astray.
- **Criterion**: In `task.md`, the construction of the harness must be completed before the logic implementation.

## 3. Proof of Non-Regression

- **SDK**: Prove logical correctness via unit tests and boundary value tests.
- **UI**: Demonstrate visual correctness through state transition tests using visual snapshots or unique IDs.

## 4. Execution Process

1.  **Define**: Confirm legal requirements using `pdf-spec-mcp`.
2.  **Plan**: Create `implementation_plan.md` and immediately persist to ELM.
3.  **Harness**: Build failing tests.
4.  **Execute**: Implement in compliance with [RR-15](RELIABLE_RUST_15.md).
5.  **Verify**: Complete `cargo test` and `verify_compliance.sh`.

## 5. Completion Gate

- **Test Passing [MUST]**: All tests for new and existing functions must PASS.
- **Evidence [MUST]**: Test results and their mapping to Clauses must be recorded in `walkthrough.md`.
- **ELM Sync [MUST]**: All records must be persisted to the project directory.
