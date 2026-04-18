# Phase 9 Retrospective: Security & Protection

## Overview
Phase 9 focused on the most critical trust-related features of the PDF specifications: Encryption and PAdES Digital Signatures. We successfully transitioned from a basic parser to a hardened, audit-ready cryptographic engine.

## Accomplishments

### 1. PAdES Digital Signatures (M24-SEC)
- **Multi-Algorithm Engine**: Implemented a swappable signature verification pipeline supporting both legacy RSA (`pkcs1v15`) and modern ECDSA (`P-256`).
- **Modification Detection (MDP)**: Successfully integrated DocMDP logic to audit document integrity in the presence of incremental updates—a features often overlooked by generic PDF libraries.
- **LTV (DSS) Deep Integration**: Built a robust resolution and caching system for Document Security Store data, enabling offline-first long-term validation.

### 2. Encryption (M23-SEC)
- **AES-256 (V=5)**: Finalized the decryption pipeline for modern PDF 2.0 documents, ensuring compliance with ISO 32000-2 Unicode-based password handling.

## Challenges & Lessons Learned

### The Inconsistency of CMS Crates
One of the primary challenges was the inconsistency of field names in different versions of the `cms` crate (e.g., `digest_alg` vs `digest_algorithm`). This necessitated deep source-code auditing and the use of shorter naming conventions to align with the crate's internal fields.

### ByteRange Accuracy
Digital signatures cover specific byte ranges. Ensuring the `ferruginous-doc` layer correctly maps these ranges across incremental updates required precise offset management. We learned that using `bytes::Bytes` for zero-copy slicing is essential for maintaining integrity without excessive memory overhead.

### Unicode in PDF Strings
PDF strings in signatures (like `Name` or `Reason`) often use UTF-16BE with a Byte Order Mark (BOM). Implementing a custom decoder was necessary to avoid data corruption in non-ASCII metadata, ensuring the tool works correctly for international documents.

## Future Improvement Areas
- **Network-based Revocation**: While LTV-first is secure, adding an optional, isolated network fetcher for live OCSP/CRL queries would increase flexibility.
- **Advanced FieldMDP Audit**: Expanding FieldMDP verification from simple "lock detection" to "value diffing" would provide even deeper structural security audits.

## Conclusion
Phase 9 has established Ferruginous as a top-tier PDF engine for security-sensitive applications. The codebase is now RR-15 compliant and ready for the next phase of Standards & Compliance.
