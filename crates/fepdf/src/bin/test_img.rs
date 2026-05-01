use ferruginous_core::Object;
use ferruginous_sdk::PdfDocument;
use std::fs;

fn main() {
    let data = fs::read("samples/nihonkokukenpou.pdf").expect("Failed to read sample PDF");
    let doc = PdfDocument::open(bytes::Bytes::from(data)).unwrap();
    let page_count = doc.page_count().unwrap();

    // Test page 2 (index 1)
    if page_count < 2 {
        println!("Document has only {} pages", page_count);
        return;
    }

    let page = doc.inner().get_page(1).unwrap();
    let arena = doc.inner().arena();

    let res_obj = page.resolve_attribute("Resources");
    let xobj_key = arena.name("XObject");

    if let Some(Object::Dictionary(rh)) = res_obj {
        if let Some(res_dict) = arena.get_dict(rh) {
            if let Some(xobj_val) = res_dict.get(&xobj_key) {
                let xobj_resolved = xobj_val.resolve(arena);
                if let Object::Dictionary(xh) = xobj_resolved {
                    if let Some(xobj_dict) = arena.get_dict(xh) {
                        for (k, v) in xobj_dict {
                            let name = arena
                                .get_name(k.clone())
                                .map(|n| n.as_str().to_string())
                                .unwrap_or_default();
                            println!("XObject: {}", name);
                            let obj = v.resolve(arena);
                            if let Object::Stream(dh, _) = obj {
                                if let Some(dict) = arena.get_dict(dh) {
                                    for (dk, dv) in dict {
                                        let dv_res = dv.resolve(arena);
                                        let dk_name = arena
                                            .get_name(dk.clone())
                                            .map(|n| n.as_str().to_string())
                                            .unwrap_or_default();
                                        let val_str = match dv_res {
                                            Object::Integer(i) => i.to_string(),
                                            Object::Name(n) => arena
                                                .get_name(n)
                                                .map(|name| name.as_str().to_string())
                                                .unwrap_or_default(),
                                            Object::Array(a) => format!(
                                                "Array of len {}",
                                                arena
                                                    .get_array(a)
                                                    .map(|arr| arr.len())
                                                    .unwrap_or(0)
                                            ),
                                            _ => format!("{:?}", dv_res),
                                        };
                                        println!("  {} = {}", dk_name, val_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
