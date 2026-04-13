# Tasks: Phase 18 Japanese Rendering

- [x] **Research & Setup**
    - [x] Create branch `feat/phase18-japanese-rendering`
    - [x] Identify and purge legacy PDFs
    - [x] Synchronized PDF Association samples (PDF 2.0)
    - [x] Create self-generated PDF 2.0 Japanese test harness (Harness Driven)
    - [x] Baseline diagnosis with `diag_layout` on generated harness
- [x] **SDK Refactoring (Track S)**
    - [x] `font.rs`: Propagate `DW2/W2` and fix `WMode` inheritance
    - [x] `text.rs`: Refactor `advance_glyph` (remove heuristics, implement Clause 9.4.4)
    - [x] `content.rs`: Implement vertical origin shift and strict width scaling
- [x] **Verification**
    - [x] Run `diag_layout` and verify matrix transformation
    - [x] Run compliance checks (`verify_compliance.sh`)
    - [x] Manual visual check of `small-ja.pdf`
- [x] **Completion**
    - [x] Merge to `main` via PR
    - [x] Sync `ROADMAP.md`
