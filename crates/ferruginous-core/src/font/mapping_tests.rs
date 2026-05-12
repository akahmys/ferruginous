#[cfg(test)]
mod tests {
    use crate::arena::PdfArena;
    use crate::font::{FontMetrics, FontResource, cmap::CMap};
    use crate::object::PdfName;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[test]
    fn test_code_to_cid_mismatch_reproduction() {
        let arena = PdfArena::new();

        // 1. Create a CMap for Encoding (Non-Identity)
        // Code 0x65 ('e') maps to CID 100
        let mut enc_cmap = CMap::default();
        let mut mappings_cid = BTreeMap::new();
        mappings_cid.insert(vec![0x65], 100);
        enc_cmap.mappings_cid = Arc::new(mappings_cid);

        // 2. Create a ToUnicode CMap
        // Code 0x65 ('e') maps to Unicode 'e'
        let mut tu_cmap = CMap::default();
        let mut mappings = BTreeMap::new();
        mappings.insert(vec![0x65], "e".to_string());
        tu_cmap.mappings = Arc::new(mappings);

        // 3. Setup FontResource
        let mut res = FontResource {
            subtype: PdfName::new("Type0"),
            base_font: PdfName::new("TestFont"),
            is_cid_keyed: true,
            encoding: Some(enc_cmap),
            to_unicode: Some(tu_cmap),
            ..FontResource::new_initial(
                PdfName::new("Type0"),
                PdfName::new("TestFont"),
                FontMetrics::default(),
                None,
                None,
                None,
                None,
                None,
                None,
                true,
                &BTreeMap::new(),
                &arena,
                false,
                None,
                None,
            )
        };

        // 4. Run build_unified_map
        res.build_unified_map();

        // 5. Verification
        // CURRENT BUG: unified_map will have "e" -> 0x65 (101)
        // EXPECTED FIX: unified_map should have "e" -> 100
        let cid = res.unified_map.get("e").copied();

        println!("Resolved CID for 'e': {:?}", cid);

        // This assertion will FAIL with the current bug (it will get 101)
        assert_eq!(
            cid,
            Some(100),
            "Unicode 'e' should map to CID 100 (via Encoding), not character code 0x65"
        );
    }
}
