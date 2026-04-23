# Skill: Friction Analysis (analyze_friction)

Analyze friction points and feedback in the development and verification loop, and reflect preventive measures in the appropriate convention files.

## 1. History Scanning

- **Rule**: Identify compilation errors, Clippy warnings, logical contradictions, or user feedback, and delve deep into the causes of trial and error.
- **Purpose**: Identify the root causes—such as flaws in the thought process or conventions—rather than making superficial fixes.
- **Criterion**: The specific occurrence points of friction and the underlying "hesitation" must be verbalized.

## 2. Categorization & Revision

- **Rule**: Categorize by type (e.g., Charter/Quality/Process) based on the [PLANNING_PROTOCOL](planning.md), and revise conventions to be "more concrete and mechanical" without compromising existing philosophy.
- **Purpose**: Feed improvements back into the SSoT and physically prevent the recurrence of similar friction.
- **Criterion**: Files to be modified are identified, and draft revisions including specific "Criteria" are created.
- **Automation Consideration**: Can the rule revision be automated through "skillization" or "workflowization"? (Recommendation: Complex rules with high frequency should be made into skills.)

## 3. Reflection (Apply)

- **Rule**: After obtaining user consensus, modify the convention files and execute [sync_docs](../workflows/sync_docs.md) to bring them up to date.
- **Purpose**: Solidify improvements as a project-wide knowledge base.
- **Criterion**: `verify_compliance.sh` passes after convention modification, and documents are synchronized.
