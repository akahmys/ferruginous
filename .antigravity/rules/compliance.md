# ISO 32000 Compliance Strategy

> [!IMPORTANT]
> **Technical compliance is the singular raison d'être of Ferruginous.**
> All implementation choices must be objectively validated against ISO 32000-2:2020 using automated auditing and differential testing.

---

## 1. Specification Mapping (Static Auditing)
- **Rule**: Every logical component MUST be mapped to specific ISO 32000-2 Clauses.
- **Verification**: Dictionary structures MUST be validated against the **Arlington PDF Model**.
- **Criterion**: `implementation_plan.md` MUST cite specific Clause numbers.

## 2. Viewer Fidelity (Dynamic Validation)
- **Rule**: Validate all generated outputs against **Adobe Acrobat Reader** as the reference baseline.
- **Verification**: Outputs must be visually and structurally correct in Acrobat. "Error 135" or internal structural warnings are considered critical compliance failures.
- **Criterion**: Pass VeraPDF "Level 2.0" validation for all generated documents.

## 3. Persistent Compliance Logs
- **Rule**: All verification results MUST be persisted to the `.antigravity/session/` directory.
- **Verification**: Every feature or fix MUST include an evidence log (clause mappings, test outputs) in its `walkthrough.md`.
- **Criterion**: Completion is defined by the existence of verified compliance evidence.

## 4. Zero-Warning Quality Gate
- **Rule**: Maintain a "Zero Tolerance" policy for static analysis warnings.
- **Verification**: 100% pass rate for `clippy` and the internal `verify_compliance.sh` suite.
- **Criterion**: The `main` branch must remain in a "Perfectly Green" state at all times.

## 5. Binary Data Sanitization
- **Rule**: Strictly isolate non-textual streams (Fonts, Images, ICCProfiles) from text-based processing.
- **Purpose**: Prevent irreversible data corruption caused by UTF-8 normalization or string-based substitution on binary payloads.
- **Compliance**: The refinery MUST utilize an explicit object-type-based denylist.

## 6. Pass 0: Physical Normalization
- **Rule**: All input documents MUST undergo "Pass 0 Physical Normalization" before semantic ingestion.
- **Purpose**: Resolve physical-layer anomalies (decryption, XRef repairs, residual `/Encrypt` dictionaries) to provide a "Plaintext PDF 2.0" baseline for the `PdfArena`.
- **Compliance**: Iterative (stack-based) walk of all raw objects is mandatory to ensure Acrobat compatibility.

## 7. Secrets & PII Guard
- **Rule**: Absolute prohibition of committing authentication tokens, keys, or PII.
- **Verification**: Mandatory execution of `verify_secrets.sh` in the compliance suite.
- **Compliance**: Any secret leak is a catastrophic failure requiring history rewriting.
