use ferruginous_core::document::Document;
use ferruginous_core::object::Object;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let doc = Document::load(std::path::Path::new(&args[1]))?;
    let arena = doc.arena();

    for i in 0..doc.arena().object_count() {
        let h = ferruginous_core::Handle::new(i as u32);
        if let Ok(obj) = doc.resolve(&h) {
            if let Object::Dictionary(dh) = obj {
                let dict = arena.get_dict(dh).unwrap();
                if let Some(subtype) = dict.get(&arena.name("Subtype")) {
                    if let Some(name) = subtype.resolve(arena).as_name() {
                        let name_str = arena.get_name(name).unwrap().as_str().to_string();
                        if name_str == "Type1" || name_str == "TrueType" || name_str == "Type0" {
                            let base_font = dict.get(&arena.name("BaseFont"))
                                .and_then(|o| o.resolve(arena).as_name())
                                .and_then(|h| arena.get_name(h))
                                .map(|n| n.as_str().to_string())
                                .unwrap_or("Unknown".to_string());
                            
                            println!("Font [{}]: {} ({})", i, base_font, name_str);
                            
                            if let Some(enc_obj) = dict.get(&arena.name("Encoding")) {
                                let resolved_enc = enc_obj.resolve(arena);
                                println!("  Encoding: {:?}", resolved_enc);
                                if let Object::Dictionary(eh) = resolved_enc {
                                    if let Some(edict) = arena.get_dict(eh) {
                                        if let Some(diffs_obj) = edict.get(&arena.name("Differences")) {
                                            let diffs_resolved = diffs_obj.resolve(arena);
                                            println!("    Differences: {:?}", diffs_resolved);
                                            if let Object::Array(ah) = diffs_resolved {
                                                if let Some(arr) = arena.get_array(ah) {
                                                    print!("      Items: ");
                                                    for item in arr {
                                                        match item.resolve(arena) {
                                                            Object::Integer(i) => print!("{} ", i),
                                                            Object::Name(nh) => print!("/{} ", arena.get_name(nh).unwrap().as_str()),
                                                            _ => {}
                                                        }
                                                    }
                                                    println!();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(fd_obj) = dict.get(&arena.name("FontDescriptor")) {
                                if let Object::Dictionary(fdh) = fd_obj.resolve(arena) {
                                    let fdict = arena.get_dict(fdh).unwrap();
                                    for key in ["FontFile", "FontFile2", "FontFile3"] {
                                        if let Some(ff) = fdict.get(&arena.name(key)) {
                                            println!("    {}: Present", key);
                                            if key == "FontFile3" {
                                                if let Object::Stream(sh, _) = ff.resolve(arena) {
                                                    let sdict = arena.get_dict(sh).unwrap();
                                                    if let Some(stype) = sdict.get(&arena.name("Subtype")) {
                                                        println!("      Subtype: {:?}", stype.resolve(arena));
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
            }
        }
    }
    Ok(())
}
