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

---

## 5. Binary Integrity Guard
- **Rule**: Non-textual streams (Fonts, Images, ICCProfiles) MUST be strictly excluded from text-based refinement pipelines.
- **Purpose**: Prevent data corruption caused by misapplying UTF-8 normalization or text substitutions to binary data.
- **Compliance Criterion**: The refinery engine must maintain an explicit "Refinement Denylist" based on PDF object types (e.g., `/FontFile`, `/XObject`).

## 6. Pass 0 Physical Normalization Guard
- **Rule**: All input PDFs MUST undergo a "Physical Normalization" phase (Pass 0) at the raw object level before semantic ingestion into the `PdfArena`.
- **Purpose**: Ensure structural integrity, viewer compatibility (e.g., Acrobat Error 135), and uniform internal state. This phase handles decryption, repair of broken XRef tables, and mandatory UTF-8 re-encoding of Metadata.
- **Compliance Criterion**: Any operation that maps physical offsets to semantic handles MUST be preceded by a call to `Ingestor::perform_pass_0_normalization`. This process must be iterative (stack-based) and non-destructive to the original document intent while stripping transport-level artifacts (like `/Encrypt` dictionaries).

## 7. Audit & Verification Procedure
- **Rule**: All significant changes must undergo a dual semantic and logical audit.
- **Purpose**: Maintain consistency and prevent implementation gaps as the codebase evolves.
- **Compliance Criterion**: Before merging any refactoring or feature, the [Verify Compliance & Structural Audit](../workflows/verify_compliance.md) workflow must be executed and all criteria met.

## 8. Confidentiality & PII Guard
- **Rule**: Accidental leakage of authentication tokens, private keys, or personal data (PII) is strictly prohibited.
- **Purpose**: Protect project security and user privacy during public synchronization.
- **Compliance Criterion**: The `verify_secrets.sh` audit MUST pass as part of the standard `verify_compliance.sh` suite. Any detection of secrets in production code or commit history must be treated as a critical compliance failure.
