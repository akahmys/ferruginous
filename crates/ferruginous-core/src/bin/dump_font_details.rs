use ferruginous_core::document::Document;
use ferruginous_core::object::Object;
use ferruginous_core::handle::Handle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: dump_font_details <pdf_file> <font_obj_id>");
        return Ok(());
    }

    let doc = Document::load(std::path::Path::new(&args[1]))?;
    let arena = doc.arena();
    let obj_id: u32 = args[2].parse()?;
    let handle = Handle::new(obj_id);

    if let Some(Object::Dictionary(dh)) = arena.get_object(handle) {
        let dict = arena.get_dict(dh).unwrap();
        println!("Font Object {}:", obj_id);
        for (name_h, obj) in dict.iter() {
            println!("  /{}: {:?}", arena.get_name(*name_h).unwrap().as_str(), obj.resolve(arena));
        }

        if let Some(enc_obj) = dict.get(&arena.name("Encoding")) {
            println!("\nEncoding details:");
            let resolved = enc_obj.resolve(arena);
            match resolved {
                Object::Name(nh) => println!("  Predefined: {}", arena.get_name(nh).unwrap().as_str()),
                Object::Dictionary(edh) => {
                    let edict = arena.get_dict(edh).unwrap();
                    if let Some(base_enc) = edict.get(&arena.name("BaseEncoding")) {
                        println!("  BaseEncoding: {:?}", base_enc.resolve(arena));
                    }
                    if let Some(diffs) = edict.get(&arena.name("Differences")) {
                        println!("  Differences: {:?}", diffs.resolve(arena));
                        if let Object::Array(ah) = diffs.resolve(arena) {
                            if let Some(arr) = arena.get_array(ah) {
                                let mut current_code = 0;
                                for item in arr {
                                    match item {
                                        Object::Integer(i) => current_code = i,
                                        Object::Name(nh) => {
                                            println!("    {} -> /{}", current_code, arena.get_name(nh).unwrap().as_str());
                                            current_code += 1;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                _ => println!("  Unknown encoding type: {:?}", resolved),
            }
        }

        if let Some(tu_obj) = dict.get(&arena.name("ToUnicode")) {
            println!("\nToUnicode stream:");
            if let Object::Stream(_, data) = tu_obj.resolve(arena) {
                let bytes = arena.get_stream_bytes(&data)?;
                println!("{}", String::from_utf8_lossy(&bytes));
            }
        }
    } else {
        println!("Object {} is not a dictionary.", obj_id);
    }

    Ok(())
}
