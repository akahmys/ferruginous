# GitHub Merge Protocol

> [!IMPORTANT]
> **Strict Governance**: The `main` branch is a protected foundation. All merges must be backed by objective evidence of compliance.

---

## 1. Linear History Policy
- **Rule**: Maintain a strictly linear Git history on `main`.
- **Method**: Use **Squash and Merge** (default) or **Rebase and Merge**. Merge commits (`--no-ff`) are prohibited on `main`.
- **Purpose**: Ensure that every commit on `main` represents a verified, atomic milestone that passes all compliance checks.

## 2. Timing of Merge (Merge Triggers)
- **Rule**: A merge from a feature/fix branch to `main` must occur ONLY when all the following conditions are met:
    1. **Task Finalization**: All items in the session-specific `task.md` are marked as `[x]`.
    2. **Zero-Warning Audit**: The `verify_compliance.sh` script returns a `PASS` status with 0 warnings.
    3. **User Approval**: The user has reviewed the `walkthrough.md` and provided explicit approval for the merge.
    4. **Linear Pre-Check**: The feature branch has been rebased onto the latest `main` to ensure a clean, linear integration.

## 3. Evidence-Based Pull Requests
- **Rule**: Every Pull Request (PR) must contain a link to the corresponding `walkthrough.md` generated during the task.
- **Requirement**: The PR description MUST include:
    1. A link to the `walkthrough.md` file in the project.
    2. A summary of the `verify_compliance.sh` audit result.
    3. Proof of visual validation (screenshots or recordings) if UI/Rendering was affected.

## 3. Mandatory CI Gate
- **Rule**: The [RR-15 Compliance Audit](file:///Users/jun/Documents/Project/Ferruginous/.github/workflows/verify_compliance.yml) GitHub Action MUST return a "Success" status before a PR can be merged.
- **Criteria**:
    - Zero `unwrap`/`expect` violations.
    - 100% test pass rate.
    - No license conflicts.

## 4. Milestone-per-Branch Lifecycle
- **Rule**: Every new feature or fix must be developed in a dedicated branch following the naming convention `feat/phaseN-...` or `fix/...`.
- **Lifecycle**:
    - `feat` branch creation -> Development -> Local Verification -> PR Submission -> CI Review -> User Review -> Squash & Merge to `main` -> Branch Deletion.

## 5. Prototype Isolation
- **Rule**: Logic from the legacy prototype (located in `.legacy/`) must be migrated through this protocol even if it was previously "complete".
- **Purpose**: Prevent the silent leak of non-compliant code from the prototype into the Rebirth foundation.
