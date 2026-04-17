# Ferruginous Rebirth Task List

- [x] Initialize new Cargo workspace
- [x] Implement `ferruginous-core`
    - [x] `types.rs`: `Object`, `Reference`, `Name` (Zero-copy)
    - [x] `lexer.rs`: Fast tokenization (Fixed comment skipping)
    - [x] `parser.rs`: Strict COS parsing (Multi-token lookahead refactor)
- [x] Implement `ferruginous-doc`
    - [x] `xref.rs`: Unified XRef management
    - [x] `catalog.rs`: Root structure
    - [x] `page.rs`: Page tree & Resources (Corrected resolution logic)
- [x] Implement `ferruginous-render`
    - [x] `bridge.rs`: `RenderBackend` trait
    - [x] `state.rs`: Graphics state logic
    - [x] `text.rs`: Multibyte/Vertical layout engine (Basic infrastructure)
    - [x] Headless image output (PNG) for verification
- [x] Implement `ferruginous-sdk`
    - [x] High-level `PdfDocument` API
- [x] Implement `ferruginous-mcp`
    - [x] `server.rs`: MCP server initialization
    - [x] `tools/`: Compliance and structural audit tools (Initial set)
- [x] Verification
    - [x] Integrate PDF Association sample tests
    - [x] Resolve `Unexpected token: DictionaryClose` in PDF parser
    - [x] Correct Page Tree resolution logic in `PdfDocument`
    - [x] Verify end-to-end rendering from `samples/Simple PDF 2.0 file.pdf` to PNG
    - [x] Validation via `pdf-spec-mcp` (Initial run successful)

## Next Tasks (Phase 7: Advanced Content)
- [ ] Implement Image Stream filters (`DCTDecode`, `CCITTFaxDecode`)
- [ ] Expand Interpreter for full Text Operator support (`BT/ET`, `Td`, `Tj`, etc.)
- [ ] Implement Form XObject resource inheritance
