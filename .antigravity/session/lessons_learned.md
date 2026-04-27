# Lessons Learned: Acrobat Compatibility & Pass 0 Normalization

## Context
During the processing of `UnicodeStandard-16.0.pdf`, we encountered **Adobe Acrobat Error 135** ("A problem occurred while reading this document"). While the file opened perfectly in macOS Preview and was logically valid, Acrobat rejected it.

## Key Discovery
1.  **`/Encrypt` Residuals**: Acrobat strictly enforces structural consistency. If a PDF is decrypted but the `/Encrypt` dictionary remains in the trailer, Acrobat attempts to use a security handler on plain data, leading to a crash.
2.  **Pass 0 Requirement**: Semantic ingestion (mapping objects to handles in the Arena) must happen on **clean, physical data**. Trying to decrypt objects "on-the-fly" during semantic mapping is fragile and can lead to partially encrypted structures (e.g., in Outlines/Bookmarks).

## Resolution: The "Pass 0 Normalization" Strategy
- **Mechanism**: Before anything else, perform an iterative (stack-based, Rule 6 compliant) walk of all raw objects.
- **Actions**:
    - Decrypt all strings and streams.
    - Remove the `/Encrypt` reference from the trailer.
    - Ensure all Metadata streams are UTF-8 compliant.
- **Result**: The internal `PdfArena` now only ever sees "Plaintext PDF 2.0" structures, regardless of the source file's physical state.

## Impact on Future Work
- **Rule 18**: Any new ingestion logic must support this normalization phase.
- **Acrobat as a Baseline**: Compliance tests MUST include a "Viewer Fidelity" check on Acrobat, as it is more pedantic than the specification itself.
