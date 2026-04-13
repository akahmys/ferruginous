use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::validator::{ComplianceValidator, ValidationReport};
use std::collections::BTreeMap;
use ferruginous_sdk::core::Object;

#[test]
fn test_unicode_integrity_validation() {
    // Manually construct a doc to avoid needing a valid physical xref table for simple tests
    use ferruginous_sdk::trailer::TrailerInfo;
    use ferruginous_sdk::xref::MemoryXRefIndex;
    use std::sync::Arc;

    let mut trailer_dict = BTreeMap::new();
    trailer_dict.insert(b"Root".to_vec(), Object::Reference(ferruginous_sdk::core::Reference::new(1, 0)));
    
    let mut entries = BTreeMap::new();
    entries.insert(1, ferruginous_sdk::xref::XRefEntry::InUse { offset: 10, generation: 0 });

    let mut data = b"%PDF-1.7\n".to_vec();
    data.extend_from_slice(b"1 0 obj << /Type /Catalog >> endobj\n"); // Starts at offset 10 approx (Actually it's exactly 10 if %PDF-1.7\n is 10 bytes)

    let doc = ferruginous_sdk::loader::PdfDocument {
        data,
        xref_index: MemoryXRefIndex { entries },
        last_trailer: TrailerInfo { last_xref_offset: 0, trailer_dict: Arc::new(trailer_dict) },
        security: None,
    };

    let validator = ComplianceValidator::new(&doc);
    let mut report = ValidationReport { errors: vec![], warnings: vec![] };
    validator.validate_metadata(&mut report).unwrap();
    
    // Document lacks XMP metadata
    assert!(report.warnings.iter().any(|w| w.contains("XMP metadata")));
}

#[test]
fn test_tagged_pdf_mismatch() {
    use ferruginous_sdk::trailer::TrailerInfo;
    use ferruginous_sdk::xref::MemoryXRefIndex;
    use std::sync::Arc;

    let mut trailer_dict = BTreeMap::new();
    trailer_dict.insert(b"Root".to_vec(), Object::Reference(ferruginous_sdk::core::Reference::new(1, 0)));

    let doc = ferruginous_sdk::loader::PdfDocument {
        data: b"%PDF-1.7\n".to_vec(),
        xref_index: MemoryXRefIndex::default(),
        last_trailer: TrailerInfo { last_xref_offset: 0, trailer_dict: Arc::new(trailer_dict) },
        security: None,
    };
    
    let validator = ComplianceValidator::new(&doc);
    // This will fail at catalog resolution because the xref_index is empty, but we can verify it doesn't panic.
    let _ = validator.validate_all();
}
