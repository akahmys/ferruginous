# Phase 18: Multibyte Text Rendering Precision

This phase focuses on correcting Japanese character positioning and rendering by implementing missing ISO 32000-2 requirements for composite fonts (Type 0) and CIDFonts.

## User Review Required

> [!IMPORTANT]
> The changes involve core rendering logic in `ferruginous-sdk`. While aiming for 100% compliance, the use of system fallback fonts on macOS (e.g., Hiragino) might lead to slight metrics mismatches if the embedded font widths in the PDF don't align with the system font. We prioritize PDF-specified widths (`W` and `W2`) over font file metrics.

## Proposed Changes

### Research & Baseline
- Run `cargo run --example create_jp_harness` to generate a test PDF.
- Run `cargo run --example diag_layout samples/pdf20/jp-harness.pdf` to establish the baseline failure state.

---

### SDK Refactoring (Track S)

#### [MODIFY] [font.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-sdk/src/font.rs)
- **Metric Propagation**: Ensure `from_type0_dict` correctly inherits and overrides metrics from the Descendant CIDFont.
- **CIDToGIDMap**: Fix the logic in `get_glyph_path` to handle `Identity` mapping correctly (CID 1:1 GID) and ensure it's applied for `CIDFontType2`.
- **Vertical Origin**: Refine `char_vertical_metrics` to ensure default values (Clause 9.7.4.3) are used when `W2` is absent.

#### [MODIFY] [text.rs](file:///Users/jun/Documents/Project/Ferruginous/crates/ferruginous-sdk/src/text.rs)
- **Origin Shift**: Update `TextState` or the rendering loop to apply the vertical origin shift `(vx, vy)` before drawing each glyph in vertical mode (`wmode == 1`).
- **FontMatrix Sync**: Ensure the 0.001 scaling factor is explicitly handled in the transformation matrix if not already included in the CTM/Tm calculation.

#### [MODIFY] [renderer.rs / content.rs] (TBD)
- Integrate the origin shift into the actual rendering loop within `ferruginous-render` or the content stream processor.

---

## Open Questions

- Should we strictly enforce the PDF's `W` and `W2` widths even if the actual font outline has a different advance? (Currently: Yes, per ISO 32000-2).

## Verification Plan

### Automated Tests
- `cargo run --example create_jp_harness`
- `cargo run --example diag_layout samples/pdf20/jp-harness.pdf`
- Check if the output shows the expected origin shift and advancement.
- `verify_compliance.sh` to ensure no `unwrap()` regressions.

### Manual Verification
- Visual inspection of the generated `jp-harness.pdf` in the `ferruginous-ui` (if possible) or system viewer.
