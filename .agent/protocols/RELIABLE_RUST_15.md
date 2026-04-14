# Reliable Rust-15 (RR-15) Rulebook

> [!IMPORTANT]
> A set of 15 items optimized from NASA's "Power of 10" for Rust. These are "Absolute Safety Constraints" for Ferruginous.

---

## 1. Function Size Limit
- **Rule**: Limit effective logic to 50 lines.
- **Purpose**: Maintain precision of the borrow checker and reduce mental load.
- **Compliance Criterion**: All functions (excluding test code) must stay within 50 lines.

## 2. No-Panic Principle
- **Rule**: Prohibit `unwrap()`, `expect()`, and `panic!()`.
- **Purpose**: Eradicate runtime crashes.
- **Compliance Criterion**: In production code, there must be no paths that cause forced termination due to unrecoverable errors.

## 3. Safety Isolation
- **Rule**: Total prohibition of `unsafe` blocks in the core layers (SDK/Render).
- **Purpose**: Maintain 100% compiler-guaranteed memory safety.
- **Compliance Criterion**: The `unsafe` keyword count must be zero in `crates/ferruginous-sdk` and `crates/ferruginous-render`.

## 4. Flat Nesting
- **Rule**: Use the `?` operator for early returns. Nest `if` statements no more than 2 levels deep.
- **Purpose**: Simplify logical paths and improve readability.
- **Compliance Criterion**: Control flow must be linear and flat, with deep indentation eliminated.

## 5. Exhaustive Match
- **Rule**: Prohibit wildcards (`_`) in `match` statements.
- **Purpose**: Induce compiler errors when enums are extended.
- **Compliance Criterion**: In all `match` expressions, the compiler must be able to detect the addition of future variants.

## 6. Stack Safety
- **Rule**: Prohibit function recursion. Use an explicit stack (`Vec`) instead.
- **Purpose**: Eradicate stack overflows.
- **Compliance Criterion**: The runtime call stack must be predictable, with no deep nested recursive calls.

## 7. Pure State Management
- **Rule**: Prohibit global mutable state (`static mut`).
- **Purpose**: Eliminate data races and execution order dependencies.
- **Compliance Criterion**: Shared state must be managed solely through explicit synchronization primitives (e.g., `Arc<Mutex<T>>`) or ownership transfers.

## 8. Type-Level Safety
- **Rule**: Leverage enums to make invalid states physically unrepresentable.
- **Purpose**: Minimize runtime conditional branches (assertions).
- **Compliance Criterion**: Guard clauses checking for "impossible states" must be minimized in the codebase.

## 9. Simplified Design
- **Rule**: Owned-First approach. Eliminate unnecessary lifetime references.
- **Purpose**: Prevent design rigidity caused by "lifetime pollution."
- **Compliance Criterion**: Structs and function signatures must be self-contained, avoiding complex lifetime annotations.

## 10. Deterministic Engineering
- **Rule**: Prohibit non-deterministic `HashMap`/`HashSet`. Use `BTreeMap`/`BTreeSet`.
- **Purpose**: Guarantee bit-perfect output for the same input.
- **Compliance Criterion**: Internal iteration order must be fixed, eliminating non-deterministic factors like hash salts.

## 11. Explicit Error Handling
- **Rule**: Prohibit `String` errors. Use concrete Enum types and the `thiserror` crate.
- **Purpose**: Ensure error traceability and programmatic recoverability.
- **Compliance Criterion**: All errors must be defined as domain-specific Enum types, allowing callers to handle them based on the type.

## 12. Bound & Invariant Enforcement
- **Rule**: Enforce a 256MB limit on external inputs (PDF streams) and explicitly state invariants using `assert!`.
- **Purpose**: Early detection of resource exhaustion (OOM) and logical contradictions.
- **Compliance Criterion**: Resource limits must be enforced by types or constants, and design assumptions must be verified within the code.

## 13. Zero Silent Swallowing
- **Rule**: Prohibit discarding errors via `.ok()` or `_`. Always log or propagate.
- **Purpose**: Early surfacing of latent bugs.
- **Compliance Criterion**: All `Result` types must be evaluated; not a single error should be ignored.

## 14. Strict Scoping
- **Rule**: Define variables just before they are used. Limit scopes (`{}`).
- **Purpose**: Minimize the lifetime of variables and prevent misinterpretation.
- **Compliance Criterion**: Variable lifetimes must be the minimum necessary, with short distances between initialization and use.

## 15. No Magic Cloning
- **Rule**: Prohibit `.clone()` for the purpose of avoiding borrow checker errors. Rethink ownership structure instead.
- **Purpose**: Visualize inefficient memory allocations and design distortions.
- **Compliance Criterion**: Use of `.clone()` must be limited to cases where "logical duplication of data" is truly required.
