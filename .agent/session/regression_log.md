# Ferruginous Regression Log

## [2026-04-18] Parser Lookahead Token Loss

### Symptom
Intermittent `PdfError::Syntactic { pos: 0, message: "Unexpected token: DictionaryClose" }` during object resolution, particularly for Page tree nodes containing arrays of integers.

### Root Cause
The `COS` parser had a single-token peek capability. In sequences like `0 0 612 396]`, the reference detection logic (`id gen R`) would consume `612` to check for `R`. When it saw `396` instead of `R`, it would push back `612`. However, a subsequent `peek()` would consume from the lexer (`396`) and store it in the single-peek slot, then the reference code would overwrite that slot with `612`, causing `396` to be permanently lost from the token stream. This led the parser to skip tokens and eventually find a delimiter (`]]`) prematurely.

### Resolution
Refactored `Parser` to use `peeked: Vec<Token>` for multiple lookahead.
- `peek_n(n)` enables inspecting any number of future tokens safely.
- No tokens are ever lost from the stream during lookahead operations.

### Verification
`cargo test -p ferruginous-sdk --test rendering_test`
Passes with 100% success on the PDF Association Sample Suite (Simple PDF 2.0).
