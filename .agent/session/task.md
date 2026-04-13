# Task: Diagnose Japanese PDF Text Rendering (Bunching/Misalignment)

- **Mode**: Fix
- **Scope**: Identify root cause of Japanese character bunching via quantitative diagnosis.
- **Session**: 2026-04-13

## Plan

1. [x] Session Initialization (Read GEMINI.md, handoff, regression_log)
2. [/] Description of Phenomenon & Hypotheses
3. [ ] Design & Execute Diagnosis (Diagnostic binary/test)
    - [ ] Observe FontMatrix, Tfs, CTM, and advancement for a sample Japanese PDF.
    - [ ] Compare W array values with calculated advancements.
4. [ ] Identify Root Cause based on evidence.
5. [ ] Design Minimal Fix (Implementation Plan).
6. [ ] Execute Fix & Verify.
7. [ ] Regression Check (All tests PASS).

## Active Step

> Step 2: Describing phenomenon and enumerating hypotheses in `task.md`.

### Phenomenon
- **Target**: Japanese PDF (Multi-byte, CIDFont Type 2 / Type 0).
- **Symptoms**: Character overlap ("bunching"), horizontal/vertical misalignment.
- **Persistence**: 4+ sessions of attempted fixes have failed or caused regressions.

### Past Failures (from regression_log)
- #1: Fixed CMap, but ignored advancement.
- #2: Fixed CTM doubling, but broke FontMatrix inheritance.
- #3: Confused scaling chain (FontMatrix vs Tfs).
- #4: Ignored W array vs font metrics conflict.

### Hypotheses
1. **[H1] Scaling Chain Inconsistency**: `FontMatrix` (0.001) is applied multiple times or in the wrong order relative to `Tfs` (font size) and the `W` array widths. Specifically, the conversion from glyph units (usually 1000-unit square) to user space is inconsistent.
2. **[H2] Advancement Coordinate Mismatch**: The horizontal displacement calculated in PDF space (bottom-up) is not correctly mapped to the Vello coordinate space (top-down), causing cumulative errors in character positioning.
3. **[H3] W vs. H/V metrics conflict**: For Japanese fonts (CIDFontType2), the PDF `W` (widths) array and `W2` (vertical) array values are not being prioritized over the embedded font's native metrics, or the conversion scaling between them is mismatched.

## On Interrupt

Session interrupted during diagnosis phase. Next AI should resume from Step 3: Design & Execute Diagnosis.
Avoid repeating the "scaling standardization" from Session 0d33f396 without proof.
