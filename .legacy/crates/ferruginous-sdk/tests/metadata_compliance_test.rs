use ferruginous_sdk::metadata::{Metadata, XmpManager};
use std::collections::BTreeMap;
use std::sync::Arc;

#[test]
fn test_xmp_generation_compliance() {
    let packet = XmpManager::generate_packet(
        Some("Ferruginous Test"),
        Some("Antigravity"),
        Some("Compliance verification for Phase 19"),
    );
    
    let xmp_str = std::str::from_utf8(&packet).unwrap();
    
    // ISO 16684-1 requirement: RDF/XML should have dc:title, dc:creator, dc:description
    assert!(xmp_str.contains("<dc:title>"));
    assert!(xmp_str.contains("<dc:creator>"));
    assert!(xmp_str.contains("<dc:description>"));
    assert!(xmp_str.contains("Ferruginous Test"));
    assert!(xmp_str.contains("Antigravity"));
}

#[test]
fn test_metadata_extraction_robustness() {
    // Mock XMP content including RDF li tags (which xmp-writer generates)
    let xmp_content = r#"
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
                    <dc:title>
                        <rdf:Alt>
                            <rdf:li xml:lang="x-default">Complex Title</rdf:li>
                        </rdf:Alt>
                    </dc:title>
                    <dc:creator>
                        <rdf:Seq>
                            <rdf:li>Author A</rdf:li>
                        </rdf:Seq>
                    </dc:creator>
                </rdf:Description>
            </rdf:RDF>
        </x:xmpmeta>
    "#;
    
    let mut dict = BTreeMap::new();
    dict.insert(b"Type".to_vec(), ferruginous_sdk::core::Object::new_name(b"Metadata".to_vec()));

    let metadata = Metadata::new(
        Arc::new(dict),
        bytes::Bytes::from(xmp_content.as_bytes().to_vec()),
    );
    
    assert_eq!(metadata.title(), Some("Complex Title".to_string()));
    assert_eq!(metadata.creator(), Some("Author A".to_string()));
}
