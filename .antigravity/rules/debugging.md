# Debugging Conventions

Conventions for troubleshooting and the debugging thought process in Ferruginous.

## 1. Hypothesis-Driven Loop
- **Rapid Hypothesis**: When facing an issue, immediately form multiple hypotheses about the cause. Do not fixate on a single possibility.
- **Fast Disproval**: Use logs and probes to determine if a hypothesis is correct or incorrect within minutes. Rapidly discarding false hypotheses is the fastest path to truth.

## 2. Visual Sincerity
- **Literal Interpretation**: Do not dismiss rendering anomalies (overlaps, offsets, incorrect order) as "mere glitches." Interpret them as mathematical facts (inverted signs, scaling mismatches, etc.).
- **Visual Clues**: Visual output is the primary debugging evidence. Infer which layer (CMap, Matrix, Scale) is at fault from the visual result.

## 3. State vs. Delta Visualization
- **Cumulative State**: Log the accumulated state (e.g., `advance_offset`) rather than just individual incremental values (Delta).
- **Temporal Tracking**: Track state changes over time to identify exactly when inconsistency (resets, reversals) occurs.

## 4. Differential Debugging
- **Reference Comparison**: Compare a "working case" against a "broken case" using identical conditions and log formats.
- **Minimization**: Identify the minimal reproduction case (specific character, font size, or rendering mode) to eliminate noise.

## 5. Pipeline Holism
- **Upstream/Downstream Check**: Ensure a fix in one layer does not break the assumptions of the Backend or contradict input from the Interpreter.
- **Unit Integrity**: Even if a fix is local, always maintain the integrity of the entire pipeline.
