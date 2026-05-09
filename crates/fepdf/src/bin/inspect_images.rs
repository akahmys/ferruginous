use ferruginous_core::document::Document;
use ferruginous_core::object::{Object, PdfName};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: inspect_images <pdf_file>");
        return Ok(());
    }

    let doc = Document::load(std::path::Path::new(&args[1]))?;
    let arena = doc.arena();

    for i in 0..arena.object_count() {
        let handle = ferruginous_core::handle::Handle::new(i as u32);
        if let Some(Object::Stream(dh, _)) = arena.get_object(handle) {
            if let Some(dict) = arena.get_dict(dh) {
                let subtype = dict
                    .get(&arena.name("Subtype"))
                    .and_then(|o| o.resolve(arena).as_name())
                    .and_then(|n| arena.get_name(n))
                    .map(|n| n.as_str().to_string());

                if subtype.as_deref() == Some("Image") {
                    println!("Image Object {}:", i);
                    for (k, v) in dict {
                        if let Some(name) = arena.get_name(k.clone()) {
                            println!("  /{}: {:?}", name.as_str(), v.resolve(arena));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
