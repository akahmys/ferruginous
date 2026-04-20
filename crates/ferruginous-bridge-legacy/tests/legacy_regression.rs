use ferruginous_bridge_legacy::lopdf::Reader;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_minimal_14_parsing() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../samples/regression/minimal_14.pdf");

    let dbg_path = path.canonicalize().unwrap_or(path.clone());
    println!("Loading test sample from: {:?}", dbg_path);

    let data = fs::read(path).expect("Failed to read test sample");

    // 1. Verify header detection
    let header_offset = Reader::find_header(&data);
    assert!(header_offset.is_some(), "Header should be found");
    assert_eq!(header_offset.unwrap(), 0);

    // 2. Verify EOF detection
    let eof_offset = Reader::find_eof(&data);
    assert!(eof_offset.is_some(), "EOF should be found");

    // 3. Verify Xref reconstruction (the "dirty" way)
    let xref = Reader::reconstruct_xref(&data);
    // Our minimal sample has 3 objects (1 0 R, 2 0 R, 3 0 R)
    assert!(xref.entries.contains_key(&1));
    assert!(xref.entries.contains_key(&2));
    assert!(xref.entries.contains_key(&3));
}

#[test]
fn test_document_load() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../samples/regression/minimal_14.pdf");

    let data = fs::read(path).expect("Failed to read test sample");
    let doc = Reader::load_document(&data).expect("Document should be loaded");

    assert_eq!(doc.version, "1.4");
    assert!(doc.xref.entries.len() >= 3);
}

#[test]
fn test_sjis_normalization() {
    use bytes::Bytes;
    use ferruginous_bridge_legacy::lopdf::{object::StringFormat, Document, Object};

    let mut doc = Document::new();
    // "日本語" in Shift-JIS: 93 FA (日) 96 7B (本) 8C EA (語)
    let sjis_bytes = vec![0x93, 0xfa, 0x96, 0x7b, 0x8c, 0xea];
    doc.objects.insert(1, Object::String(Bytes::from(sjis_bytes), StringFormat::Literal));

    doc.apply_normalization();

    if let Some(Object::String(normalized, _)) = doc.get_object(1) {
        let s =
            std::str::from_utf8(normalized).expect("Normalization should result in valid UTF-8");
        assert_eq!(s, "日本語");
    } else {
        panic!("Object not found or wrong type");
    }
}

#[test]
fn test_repair_audit_log() {
    // Create a heavily corrupted but recoverable file
    let corrupted = b"Gunk%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\nstartxref\n0\n%%EOF";

    let mut reader = Reader::load_document(corrupted).expect("Failed to load corrupted document");
    reader.apply_normalization();

    // Print log for manual verification
    for entry in &reader.repair_log {
        println!("[REPAIR] {}", entry);
    }

    // Verify key audit events are logged
    assert!(reader.repair_log.iter().any(|l| l.contains("Repairing start position")));
    assert!(reader.repair_log.iter().any(|l| l.contains("Reconstructing xref table")));
    assert!(reader.repair_log.iter().any(|l| l.contains("Successfully recovered trailer")));
    assert!(reader.repair_log.iter().any(|l| l.contains("Starting character normalization")));
}

#[test]
fn test_corruption_recovery() {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.push("../../samples/malformed/");

    // 1. No Header
    let data_no_header = fs::read(root.join("no_header.pdf")).unwrap();
    let doc_no_header =
        Reader::load_document(&data_no_header).expect("Should recover no_header.pdf");
    assert_eq!(doc_no_header.version, "1.4");

    // 2. Broken Xref Offset
    let data_broken_xref = fs::read(root.join("broken_xref_offset.pdf")).unwrap();
    let doc_broken_xref =
        Reader::load_document(&data_broken_xref).expect("Should recover broken_xref_offset.pdf");
    assert!(doc_broken_xref.objects.contains_key(&1));
}
