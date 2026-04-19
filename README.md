# Ferruginous: The Professional PDF SDK

**Ferruginous** is a high-performance, ISO 32000-2 (PDF 2.0) compliant engine and SDK for advanced PDF manipulation. This project includes **fepdf**, the official universal CLI toolkit powered by the Ferruginous SDK.

## fepdf CLI Tool

**fepdf** is a subcommand-based multi-tool that exposes the power of the Ferruginous engine for compliance, optimization, and document forensics.

- **Diagnostic Dashboard (`info`)**: Deep metadata analysis and structural visualization.
- **Compliance Audit (`info --audit`)**: Automated verification for PDF/A-4, PDF/X-6, and PDF/UA-2.
- **Standard-First Upgrade (`upgrade`)**: Seamless conversion to modern PDF 2.0 standards.
- **Object Cloning Engine (`merge` / `split`)**: Recursive ID re-indexing for safe document merging and page extraction.
- **Scavenger Mode (`repair`)**: Reconstructs corrupted XRef tables and salvages malformed documents.
- **Optimization (`optimize`)**: Object garbage collection, metadata stripping, and encryption.
- **High-Fidelity Rendering (`render`)**: Vello-powered GPU rendering of PDF content to high-resolution images.
- **CJK Font Hardening**: Production-ready support for Japanese (mojibake-free) rendering via robust ToUnicode translation and fallback decoders.
- **Page Operations (`rotate`)**: Batch orientation adjustments.

## Installation

### From Source
```bash
make build-local
```
Binaries will be available in `target/release/fepdf`.

### Multi-platform Builds
To generate binaries for Mac (Silicon/Intel), Windows, and Linux:
```bash
make dist
```

## Usage

```bash
# Analyze document compliance
fepdf info --audit input.pdf

# Merge multiple files into a PDF 2.0 document
fepdf merge doc1.pdf doc2.pdf -o combined.pdf

# Repair a corrupted file
fepdf repair broken.pdf -o fixed.pdf

# Strip metadata and remove unreachable objects
fepdf optimize input.pdf --strip --vacuum -o clean.pdf
```

## Architecture

fepdf is built upon a modular multi-crate architecture:
- `ferruginous-core`: Low-level PDF primitives and zero-copy parser.
- `ferruginous-doc`: High-level document object model and XRef management.
- `ferruginous-sdk`: Orchestration layer for complex manipulation (Cloning, Merging).
- `fepdf`: The unified CLI entry point.

## License

- Licensed under the **MIT License**.
- ISO 32000-2:2020 Compliant.
