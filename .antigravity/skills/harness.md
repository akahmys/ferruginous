# Harness-Driven Development (HDD) Protocol

> [!IMPORTANT]
> **Build Mode**: Prioritize "Mechanical Proof." Construct a verification environment (Harness) prior to implementation to dynamically prove consistency with ISO 32000-2.

---

## 1. Specification-First Design
- **Rule**: Prior to implementation, extract requirements (shall/must) from ISO 32000-2 using `pdf-spec-mcp`.
- **Purpose**: Eliminate developer subjectivity and ensure "Full Compliance" at the architectural stage.
- **Compliance Criterion**: `implementation_plan.md` must cite specific ISO Clauses and their requirements.

## 2. Harness-First Implementation
- **Rule**: Build a failing test or a diagnostic probe *before* implementing the production logic.
- **Purpose**: Confirm that the verification mechanism correctly reflects the specification and prevents "lost-at-sea" debugging.
- **Compliance Criterion**: `task.md` must list harness completion as a prerequisite for logic implementation.

## 3. Proof of Non-Regression
- **Rule**: Every new feature must include unit tests for logical core and integration tests for document-wide side effects.
- **Purpose**: Guarantee that new features do not destabilize existing foundational layers.
- **Compliance Criterion**: All existing tests in the workspace must PASS, with 100% pass rate in `cargo test`.

## 4. Evidence Persistence
- **Rule**: Persist all test logs and proof-of-compliance artifacts to the project's permanent record (`walkthrough.md`).
- **Purpose**: Create a traceable audit trail for ISO compliance and feature stability.
- **Compliance Criterion**: A feature is defined as [Complete] only when its evidence is documented in the project's ELM (External Long-Term Memory). For graphics/rendering, "Visual Proof" (e.g., screenshots or PNGs) is a mandatory requirement.

## 5. Atomic Interface Compliance
- **Rule**: When a shared trait (e.g., `RenderBackend`) or public interface is modified, all implementations (Mock, Vello, SDK etc.) and their call-sites MUST be updated within the same task block.
- **Purpose**: Prevent "broken state" commits and minimize iterative compilation loops.
- **Compliance Criterion**: `cargo check --all` must pass after every interface-modifying task.

## 6. Fail-Fast Integration
- **Rule**: For features involving complex context propagation (e.g., Resolvers, Resource Stacks), a basic integration test MUST be executed early in the harness phase.
- **Purpose**: Detect architectural misalignments before deep implementation.
- **Compliance Criterion**: `rendering_test` must be confirmed green for basic context initialization before proceeding beyond the harness phase.
