# Skill: Scaffold Harness (scaffold_harness)

Autonomously construct a "failing verification environment" prior to implementation, based on the [HDD_PROTOCOL](harness.md).

## 1. Specification Definition

- **Rule**: Strictly define expected inputs and outputs based on `specs/` or ISO Clause information.
- **Purpose**: Determine the "correct answer" before implementation to eliminate hesitation during development.
- **Criterion**: The Clause number must be included in the test name or comments.

## 2. Proving Failure (Fail-First)

- **Rule**: Run `cargo test` with empty logic and confirm that it fails as intended.
- **Purpose**: Prove that the test itself functions correctly (retains verification capability).
- **Criterion**: An expected error (Panic or Assertion Fail) is recorded in the test execution results.
