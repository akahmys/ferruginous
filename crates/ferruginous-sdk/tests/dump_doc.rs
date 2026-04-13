use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::core::{Object, Resolver};
use std::path::Path;

#[test]
fn test_dump_c2_1() {
    let pdf_path = "../../samples/japanese-test.pdf";
    let data = std::fs::read(pdf_path).unwrap();
    let doc = load_document_structure(&data).unwrap();
    
    for i in 1..2000 {
        let reference = ferruginous_sdk::core::Reference { id: i, generation: 0 };
        if let Ok(obj) = doc.resolver().resolve(&reference) {
            if let Object::Dictionary(dict) = &obj {
                if let Some(Object::Name(n)) = dict.get(b"Type".as_ref()) {
                    if **n == *b"Font" {
                        let subtype = dict.get(b"Subtype".as_ref()).and_then(|o| {
                            match o {
                                Object::Reference(r) => doc.resolver().resolve(r).ok(),
                                _ => Some(o.clone()),
                            }
                        }).and_then(|o| {
                            match o {
                                Object::Name(s) => Some(String::from_utf8_lossy(&s).into_owned()),
                                _ => None,
                            }
                        });
                        if subtype.as_deref() == Some("Type0") {
                            println!("Font Obj {}: {:?}", i, dict.keys().map(|k| String::from_utf8_lossy(k).into_owned()).collect::<Vec<_>>());
                            if let Some(df) = dict.get(b"DescendantFonts".as_slice()) {
                                println!("  DescendantFonts raw: {:?}", df);
                                let resolved = doc.resolver().resolve(if let Object::Reference(r) = df { r } else { &ferruginous_sdk::core::Reference{id:0,generation:0} });
                                println!("  DescendantFonts resolved: {:?}", resolved);
                            } else {
                                println!("  NO DESCENDANT FONTS!");
                            }
                        }
                    }
                }
            }
        }
    }
}
