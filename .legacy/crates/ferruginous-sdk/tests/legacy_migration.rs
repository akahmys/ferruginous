#[cfg(feature = "legacy")]
mod tests {
    use ferruginous_bridge_legacy::lopdf::{Document as LegacyDocument, Object as LegacyObject, Reader};
    use ferruginous_sdk::migration::{migrate_legacy_object, align_legacy_pages};
    use ferruginous_sdk::core::types::Object;
    use bytes::Bytes;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_full_migration_flow() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../../samples/legacy/minimal_14.pdf");
        
        let data = fs::read(path).expect("Failed to read test sample");
        
        // 1. Bridge Layer: Load and Repair
        let mut legacy_doc = Reader::load_document(&data).expect("Bridge failed to load document");
        legacy_doc.apply_normalization();

        // 2. Migration Layer: Convert to Modern SDK types
        let root_ref = legacy_doc.trailer.get(b"Root".as_slice()).expect("Missing Root in trailer");
        let resolved_legacy = legacy_doc.resolve(root_ref);
        println!("Resolved legacy root type: {:?}", resolved_legacy);
        let modern_root = migrate_legacy_object(resolved_legacy.clone());
        println!("Migrated modern root type: {:?}", modern_root);

        // Verify modern object structure
        if let Object::Dictionary(dict) = modern_root {
            assert!(dict.contains_key(b"Type".as_ref()));
            assert_eq!(dict.get(b"Type".as_ref()).and_then(|o| o.as_str()), Some(b"Catalog".as_slice()));
        } else {
            panic!("Migrated root is not a dictionary (Actual: {:?})", modern_root);
        }

        // 3. Page Tree Alignment
        let pages = align_legacy_pages(&legacy_doc);
        assert!(!pages.is_empty());
        println!("Successfully aligned {} pages from legacy document", pages.len());
    }
}
