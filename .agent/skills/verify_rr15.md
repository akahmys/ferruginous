# Skill: RR-15 Compliance Audit (verify_rr15)

Audit the codebase from multiple perspectives to ensure compliance with [RELIABLE_RUST_15](../protocols/RELIABLE_RUST_15.md) and identify any non-compliant areas.

## 1. Static & Mechanical Audit

Execute the following actions to extract mechanical non-compliances:

- **Action**: Run `./scripts/verify_compliance.sh`.
- **Purpose**: Immediately detect function length, unwrap/expect, unsafe blocks, static mut, and the use of non-deterministic collections like HashMap.
- **Note**: If the script yields an error, fixing it must be the top priority.

## 2. Logical & Architectural Audit

Leverage the AI's contextual understanding to identify violations that are difficult for scripts to detect:

- **Rule 4 (Nesting)**: Are there overlapping complex `if let` or `match` statements? Is there room for flattening using early returns with the `?` operator?
- **Rule 6 (Recursion)**: Are there any recursive calls (including indirect ones)? Can they be converted to loops with an explicit stack (`Vec`)?
- **Rule 8 (Invalid State)**: Can the logic be expressed using type-safe Enums (State machine) instead of `Option` or `Result`?
- **Rule 15 (Cloning)**: Is that `.clone()` truly necessary? Can it be resolved through sharing with `Arc` or by redesigning to use ownership transfer?

## 3. Reporting Audit Results

If violations are found during the audit, report them in the following format:

1.  **Violation Location**: Filename and line number.
2.  **Violated Rule**: RR-15 rule number.
3.  **Recommended Fix**: A fix including concrete code snippets.

## 4. Completion Criterion

- [ ] `verify_compliance.sh` PASSES.
- [ ] `cargo clippy --pedantic` yields no warnings.
- [ ] In the logical audit above, it is reasonably judged that there is no further room for improvement.
