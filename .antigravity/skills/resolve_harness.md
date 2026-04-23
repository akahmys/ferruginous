# Skill: Resolve Harness (resolve_harness)

Bring a constructed verification harness to a PASS state through an implementation that complies with [RR-15](../rules/hardening.md).

## 1. Minimal Logic

- **Rule**: Write only the minimum necessary logic to make the harness PASS.
- **Purpose**: Eliminate unnecessary complexity and maintain an accurate implementation of the specifications.
- **Criterion**: Tests become Green and no redundant functionality is included.

## 2. Ownership & Borrowing

- **Rule**: Prohibit resolving borrowing errors using `.clone()`; instead, redesign the ownership structure itself based on RR-15.
- **Purpose**: Ensure memory efficiency, runtime performance, and the integrity of the design.
- **Criterion**: No unnecessary clones are reported in static analysis.
