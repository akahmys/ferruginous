use ferruginous_core::document::Document;
use ferruginous_core::object::Object;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: inspect_fonts <pdf_file>");
        return Ok(());
    }

    let doc = Document::load(std::path::Path::new(&args[1]))?;
    let arena = doc.arena();

    for i in 0..arena.object_count() {
        let handle = ferruginous_core::handle::Handle::new(i as u32);
        if let Some(Object::Dictionary(dh)) = arena.get_object(handle) {
            let dict = arena.get_dict(dh).unwrap();
            let subtype = dict
                .get(&arena.name("Subtype"))
                .and_then(|o| o.resolve(arena).as_name())
                .and_then(|n| arena.get_name(n))
                .map(|n| n.as_str().to_string());

            if subtype.as_deref() == Some("Type0")
                || subtype.as_deref() == Some("CIDFontType2")
                || subtype.as_deref() == Some("TrueType")
                || subtype.as_deref() == Some("Type3")
            {
                println!("Font Object {}:", i);
                for (k, v) in &dict {
                    let name = arena.get_name(k.clone()).unwrap();
                    let resolved = v.resolve(arena);
                    println!("  /{}: {:?}", name.as_str(), resolved);

                    if name.as_str() == "FontDescriptor" {
                        if let Some(fd_h) = resolved.as_dict_handle() {
                            let fd_dict = arena.get_dict(fd_h).unwrap();
                            for (fk, fv) in fd_dict {
                                let f_name = arena.get_name(fk).unwrap();
                                let f_resolved = fv.resolve(arena);
                                println!("    /{}: {:?}", f_name.as_str(), f_resolved);
                                if f_name.as_str().starts_with("FontFile") {
                                    if let Some(Object::Stream(sh, _)) = Some(f_resolved) {
                                        let s_dict = arena.get_dict(sh).unwrap();
                                        println!("      (stream dict): {:?}", s_dict);
                                    }
                                }
                            }
                        }
                    }
                    if name.as_str() == "CharProcs" {
                        if let Some(cp_h) = resolved.as_dict_handle() {
                            let cp_dict = arena.get_dict(cp_h).unwrap();
                            print!("    (keys): ");
                            for ck in cp_dict.keys() {
                                let c_name = arena.get_name(*ck).unwrap();
                                print!("/{} ", c_name.as_str());
                            }
                            println!();
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
