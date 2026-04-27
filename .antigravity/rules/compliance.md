# ISO 32000 Compliance Strategy

> [!IMPORTANT]
> **Compliance with standards is the raison d'être of Ferruginous.**
> Eliminate subjectivity and objectively prove full compliance using MCP tools, the Arlington Model, and differential testing.

---

## 1. Static Verification (Specification Comparison)

- **Rule**: Map every logic block to specific ISO 32000-2 Clauses and verify dictionary structures against the Arlington PDF Model.
- **Purpose**: Eliminate developer subjectivity and ensure "Full Compliance" at the structural level.
- **Criterion**: The `implementation_plan.md` must cite Clause numbers, and `PdfRegistry` lookups must return valid Arlington definitions.

## 2. Dynamic Verification (Rendering & Behavior)

- **Rule**: Validate generated/rendered output against external reference implementations and compliance suites (e.g., VeraPDF).
- **Purpose**: Ensure that the logical implementation results in a byte-perfect or visually consistent standard-compliant PDF.
- **Criterion**: Rendering results must match reference baselines within a defined pixel tolerance, and generated files must pass VeraPDF "Level 2.0" validation.
- **Fidelity Standard**: All generated PDFs must be compatible with Adobe Acrobat Reader (current version). Errors like "Error 135" or structural warnings during document opening are considered compliance failures.

## 3. Persistent Evidence Log

- **Rule**: Persist all verification results (logs, clause mappings, test outputs) to ELM in `walkthrough.md`.
- **Purpose**: Maintain a permanent, traceable record of compliance for every feature or fix.
- **Criterion**: A feature is not complete until its `walkthrough.md` contains successful test logs and explicit Clause references.

## 4. Zero-Warning Quality Gate

- **Rule**: Maintain zero warnings for all static analysis tools (Clippy, Rustc) and 100% pass rate for `verify_compliance.sh`.
- **Purpose**: Ensure code health and prevent the accumulation of "technical debt" or security vulnerabilities.
- **Criterion**: CI/CD (or manual `verify_compliance.sh`) must return success for the `main` branch at all times.
