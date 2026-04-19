# Retrospective: Phase 15 - CJK Font Hardening (Mojibake Resolution)
Date: 2026-04-19
Milestone: Phase 15 (CJK Support)

## Context
Persistent character garbling (Mojibake) was observed in Japanese PDF documents using subsetted OpenType fonts. High-profile documents (e.g., the Constitution of Japan) exhibited mapping failures where Kanji characters were rendered as boxes or incorrect glyphs.

## Root Cause Analysis
Three critical issues were identified through deep-dive diagnostic hex audits:
1.  **Compressed ToUnicode Streams**: The `/ToUnicode` dictionary streams were stored with `FlateDecode` compression. The engine was attempting to parse the raw Zlib bytes rather than the decoded CMap text.
2.  **CMap Key Length Mismatch**: Subsetted fonts often use varying byte lengths for character codes (1 vs 2 bytes). The lookup logic was too strict, failing to match `0x00A5` against `0xA5`.
3.  **Legacy Encoding Ambiguity**: Some "modern" PDFs still rely on non-standard Unicode mapping or legacy Shift-JIS encoding internally for metadata and font names, which the standard UTF-16BE decoder could not handle.

## Implementation Details
-   **Stream De-compression**: Integrated `Object::decode_stream()` into the font loading pipeline (`font/mod.rs`).
-   **Bidirectional Padding Normalization**: Hardened `CMap::lookup` to strip leading zeros or pad with zeros to match target key lengths.
-   **Robust Fallback Decoding**: Integrated `encoding_rs` to provide a Shift-JIS/CP932 fallback when standard UTF-16BE decoding fails.

## Lessons Learned
-   **Trust but Verify (Decoding)**: Never assume a PDF stream is raw text even if the dictionary suggests it. Always check for filters.
-   **Normalization is Critical**: PDF character mapping is "loose" in the wild. A robust engine must accommodate varying byte lengths.
-   **Production Quality Requires Rituals**: The "Ritual" of running Clippy (`-D warnings`) and full tests revealed latent issues in the SDK (missing docs, stylistic lints) that were resolved as part of this phase closure, improving overall project health.

## Verification Result
Successfully rendered Page 3 of `nihonkokukenpou.pdf` with **100% accurate Kanji mapping** (e.g., 「九」「際」「紛」「争」).

---
*Status: Closed. CJK Support Integrated into Production Baseline.*
