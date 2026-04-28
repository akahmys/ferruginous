# Reliable Rust-15 (RR-15) Rulebook

> [!IMPORTANT]
> A set of 15 items optimized from NASA's "Power of 10" for Rust. These are "Absolute Safety Constraints" for Ferruginous.

---

## 1. Function Size Limit
- **Rule**: Limit effective logic to 50 lines.
- **Purpose**: Maintain precision of the borrow checker and reduce mental load.
- **Compliance Criterion**: All functions (excluding test code) must stay within 50 effective logic lines. Blank lines, doc-comments, and single-line attribute macros (like `#[test]`) are excluded from this count.

## 2. No-Panic Principle
- **Rule**: Prohibit `unwrap()`, `expect()`, and `panic!()` for input-driven errors.
- **Purpose**: Eradicate runtime crashes caused by malformed or unexpected data.
- **Compliance Criterion**: In production code, there must be no paths that cause forced termination due to unrecoverable errors originating from external inputs. Use `Result` for all data-dependent operations.

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

## 6. Stack Safety (Unbounded Recursion Guard)
- **Rule**: Prohibit unbounded function recursion. Use an explicit stack (`Vec`) for structural traversals.
- **Purpose**: Eradicate stack overflows and ensure compliance with ISO 32000-2 resource limits.
- **Compliance Criterion**: Any function traversing tree-like structures (e.g., page trees or object graphs) MUST either:
    1. Use an explicit `Vec`-based stack for iterative traversal.
    2. Use a `depth: usize` parameter with a hard-coded safety limit (e.g., 32 or 64).

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
- **Compliance Criterion**: All errors must be defined as domain-specific Enum types. Complex variants MUST use **named fields** (struct-like variants) rather than tuples. All string-based payloads MUST use `std::borrow::Cow<'static, str>` to allow zero-copy static messages and efficient dynamic context. Errors must capture mandatory contextual enrichment (e.g., `pos` for parsing, `context` for ingestion).

## 12. Bound & Invariant Enforcement
- **Rule**: Enforce a 256MB limit on external inputs and explicitly state logical invariants using `assert!`.
- **Purpose**: Early detection of resource exhaustion and developer-originated logical contradictions.
- **Compliance Criterion**: Resource limits must be enforced by types or constants. `assert!` MUST ONLY be used for "impossible" logical states that indicate a bug in the code, never for validating untrusted input.

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
