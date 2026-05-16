# Debugging Conventions

Standard procedures for rapid troubleshooting and mathematical verification of rendering anomalies.

## 1. Hypothesis-Driven Verification
- **Rapid Branching**: Formulate multiple causes (Hypotheses) immediately. Do not fixate on a single path.
- **Fast Disproval**: Prioritize probes that can disprove a hypothesis within minutes. Discarding false leads is the fastest route to the root cause.

## 2. Visual Sincerity
- **Anomalies as Evidence**: Never dismiss rendering glitches as "artifacts." Treat them as mathematical proofs of sign errors, scaling mismatches, or state-machine failures.
- **Clue Inference**: Infer the faulty layer (CMap, Matrix, or Buffer) directly from visual evidence (e.g., mojibake implies CMap, offsets imply Matrix).

## 3. State Visualization
- **Cumulative State**: Always log the **Total Accumulated State** (e.g., current CTM, total advance) rather than incremental deltas to identify drift over time.
- **Inconsistency Tracking**: Monitor state resets and reversals to pinpoint the exact operator causing state corruption.

## 4. Differential Debugging
- **Ground Truth Comparison**: Compare "Working" vs. "Broken" cases using identical conditions and log formats.
- **Reproduction Minimization**: Isolate the smallest possible reproduction case (single character, specific font size) to eliminate noise.

## 5. Layer Isolation
- **Physical vs. Semantic**:
    - **Physical**: Decryption, stream decompression, parsing. (Symptoms: Corrupt bytes, syntax errors).
    - **Semantic**: Refinement, resource mapping, sublimation. (Symptoms: Invisible text, incorrect font face, mojibake).

## 6. Raw Data Verification
- **Ground Truth**: Always verify rendering bugs against the **Raw PDF Byte Stream** before trusting the Intermediate Representation (IR). Buggy sublimation logic can mask the underlying issue.

## 7. Tool-Chain Discipline
- **Command Verification**: When utilizing project-specific CLI tools (e.g., `fepdf`), always verify the command signature using `--help` before execution. Do not rely on positional inference or memory for complex argument structures.
- **Atomic Modification**: Prefer atomic, single-file edits when updating sensitive configuration or rule documents to prevent broad failure modes in AI-driven processing.

## 8. External Structural Validation
- **Mutool Audit**: Use `mutool info` and `mutool clean` to verify page tree integrity and resource reachability. A reported page count of 0 or missing fonts indicates a structural partitioning failure in Section 2 or Section 6.
- **QPDF Compliance**: Use `qpdf --check` for authoritative linearization and Xref/Trailer linkage validation. "File is linearized" must be the result.
- **Bitstream Verification**: Pay close attention to "overflow reading bit stream" warnings in `qpdf`; these indicate bit-level alignment errors in hint tables that can cause Fast Web View failures.
