# Reflection & Protocol Evolution Protocol

> [!IMPORTANT]
> **Continuous Improvement**: This protocol defines how Ferruginous learns from its own development friction to harden rules and workflows.

---

## 1. Structured Retrospective
- **Rule**: Every development cycle (Phase, Epic, or major Feature) MUST conclude with a post-mortem analysis of failures and iterations.
- **Purpose**: Bridge the gap between "momentary mistakes" and "systemic safeguards."
- **Compliance Criterion**: A `retrospective` entry must be added to `.antigravity/reflections.md` citing specific tool/workflow failures.

## 2. Rule Distillation (R-P-C Format)
- **Rule**: Identified friction points must be evaluated for "Rule Potential." If a friction point is structural, it MUST be converted into a rule using the **Rule, Purpose, Criterion** format.
- **Purpose**: Systematically eliminate classes of errors rather than just patching them.
- **Compliance Criterion**: New protocols must be validated against existing RR-15 and HDD standards for consistency.

## 3. Workflow Feedback Loop
- **Rule**: If a workflow (e.g., `initialize_harness.md`) consistently leads to downstream failures, the workflow itself MUST be modified to include preventative steps.
- **Purpose**: Optimize the AI-developer collaboration for maximum reliability.
- **Compliance Criterion**: Updates to workflows must be verified through the next execution cycle.

## 4. Architectural Validation (Phase Closure)
- **Rule**: Every phase MUST include a step for validating core data structures and data flows to ensure they adhere to RR-15 and HDD principles.
- **Purpose**: Prevent architectural drift and accumulate technical debt.
- **Compliance Criterion**: A visual check (e.g., Mermaid diagrams) or static analysis report of the changed components must be reviewed and documented in the phase walkthrough.
