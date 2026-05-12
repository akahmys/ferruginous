# PDF Production & Optimization Protocol

> [!IMPORTANT]
> **Production Standard**: All PDF outputs intended for distribution MUST target PDF 2.0 and utilize high-density optimization features to ensure ISO 32000-2 compliance and efficient delivery.

---

## 1. PDF 2.0 Upgrade Strategy
- **Rule**: Explicitly target PDF 2.0 (ISO 32000-2) unless a legacy standard (e.g., PDF/A-1) is specifically requested.
- **Implementation**: Use the `fepdf produce upgrade` command.
- **Key Parameters**:
    - `--string-encoding utf8`: Required for PDF 2.0 to support modern internationalized strings.
    - `--lang <LANG>`: Must be set to the document's primary language (e.g., `ja-JP`, `en-US`) to support accessibility and search.

## 2. High-Density Optimization (Precipitation)
- **Rule**: Apply the following "Full Optimization" stack for all production releases:
    - **`--obj-stm`**: Consolidate logical objects into Object Streams to reduce file size and improve parsing efficiency.
    - **`--compress`**: Apply FlateDecode compression to all streams.
    - **`--vacuum`**: Perform unreachable object elimination to remove orphans and legacy data.
    - **`--strip`**: Remove non-essential descriptive metadata (Info dictionary) in favor of authoritative XMP metadata.

## 3. Web Delivery & Linearization
- **Rule**: Enable Fast Web View for documents intended for web consumption.
- **Implementation**: Add the `--linearize` flag.
- **Constraint**: Linearization requires a final two-pass writing process to generate hint tables; ensure sufficient scratch space is available.

## 4. Metadata Integrity
- **Rule**: Ensure Metadata (XMP) is synchronized with document attributes.
- **Parameters**:
    - `--title "<TITLE>"`: Override if the original title is missing or corrupt.
    - `--author "<AUTHOR>"`: Ensure correct attribution for the generated document.

## 5. Verification of Output
- **Rule**: Verify the optimized output using the `analyze` subcommand before delivery.
- **Compliance Criterion**: `fepdf analyze <OUTPUT>` must show 0 Critical errors and confirm PDF 2.0 versioning.

## 6. Output Management
- **Rule**: All generated PDF artifacts MUST be stored in the `artifacts/` directory at the project root.
- **Purpose**: Centralize production results for easy access, verification, and distribution.

---

### Standard Production Command Template
```bash
./target/debug/fepdf produce upgrade \
  --linearize \
  --compress \
  --vacuum \
  --strip \
  --obj-stm \
  --string-encoding utf8 \
  --lang ja-JP \
  <INPUT_PDF> artifacts/<OUTPUT_FILENAME>.pdf
```
