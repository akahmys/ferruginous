use ferruginous_core::{Document, Handle, Object, SublimatedData};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: find_page <pdf_path> <page_number>");
        return Ok(());
    }
    let path = Path::new(&args[1]);
    let page_idx_str = &args[2];
    let doc = Document::load(path)?;
    println!("Document loaded. Args: {:?}", args);

    fn dump_resolved(doc: &Document, resolved: Object) {
        match resolved {
            Object::Reference(h) => {
                println!("  Reference({:?})", h);
                let resolved = doc.arena().get_object(h).unwrap();
                println!("  Resolved Type: {:?}", resolved);
                dump_resolved(doc, resolved);
            }
            Object::Stream(dh, ref sh) => {
                println!("  Stream Object:");
                let d = doc.arena().get_dict(dh).unwrap();
                println!("    Stream Dictionary:");
                for (nk, val) in &d {
                    let name = doc.arena().get_name(nk.clone()).unwrap();
                    println!("      /{} -> {:?}", name.as_str(), val);
                }
                match sh.as_ref() {
                    SublimatedData::Image { width, height, format, .. } => {
                        println!("    Sublimated Image: {}x{}, format={:?}", width, height, format);
                    }
                    SublimatedData::Commands(cmds) => {
                        println!("    Sublimated Commands: {} ops", cmds.len());
                    }
                    SublimatedData::Compressed { original_len, .. } => {
                        println!("    Compressed Data: {} bytes", original_len);
                    }
                    SublimatedData::Raw(bytes) => {
                        println!("    Raw Data: {} bytes", bytes.len());
                    }
                }
                let raw_data = doc.arena().get_stream_bytes(sh).unwrap();
                let data = doc.arena().process_filters(&raw_data, &d).unwrap_or(raw_data);
                let data_slice: &[u8] = &data;
                println!("    Stream Content (first 100 bytes): {:?}", &data_slice[..std::cmp::min(100, data_slice.len())]);
                if let Ok(s) = std::str::from_utf8(data_slice) {
                     if s.len() < 1000 {
                         println!("    Stream Content (UTF8): \n{}", s);
                     } else {
                         println!("    Stream Content (UTF8): (long, {} bytes)", s.len());
                     }
                }
                if std::env::var("EXPORT_STREAM").is_ok() {
                    let filename = format!("stream_{}.bin", dh.index());
                    std::fs::write(&filename, data_slice).unwrap();
                    println!("    Stream exported to {} (decompressed)", filename);
                }
            }
            Object::Dictionary(dh) => {
                let d = doc.arena().get_dict(dh).unwrap();
                println!("  Dictionary:");
                for (nk, val) in d {
                    let name = doc.arena().get_name(nk.clone()).unwrap();
                    println!("    /{} -> {:?}", name.as_str(), val);
                }
            }
            Object::Array(h) => {
                let a = doc.arena().get_array(h).unwrap();
                println!("  Array:");
                for (i, val) in a.iter().enumerate() {
                    println!("    [{}] -> {:?}", i, val);
                }
            }
            _ => println!("  {:?}", resolved),
        }
    }

    if page_idx_str.starts_with("obj:") {
        let sub = &page_idx_str[4..];
        let handle_val = sub.parse::<u32>()?;
        let handle = Handle::<Object>::new(handle_val);
        println!("Dumping Object Handle: {:?}", handle);
        dump_resolved(&doc, Object::Reference(handle));
        return Ok(());
    } else if page_idx_str.starts_with("dict:") {
        let id = page_idx_str[5..].parse::<u32>()?;
        println!("Dumping Dictionary {}:", id);
        dump_resolved(&doc, Object::Dictionary(Handle::new(id)));
        return Ok(());
    } else if page_idx_str.starts_with("arr:") {
        let id = page_idx_str[4..].parse::<u32>()?;
        println!("Dumping Array {}:", id);
        dump_resolved(&doc, Object::Array(Handle::new(id)));
        return Ok(());
    } else if page_idx_str.starts_with("name:") {
        let id = page_idx_str[5..].parse::<u32>()?;
        println!("Dumping Name {}:", id);
        if let Some(name) = doc.arena().get_name(Handle::new(id)) {
            println!("  {}", name.as_str());
        } else {
            println!("  Name not found");
        }
        return Ok(());
    } else if page_idx_str.starts_with("obj:") {
        let id = page_idx_str[4..].parse::<u32>()?;
        println!("Dumping Object {}:", id);
        let obj = doc.arena().get_object(Handle::new(id)).unwrap_or(Object::Null);
        println!("  {:?}", obj);
        if let Object::Reference(h) = obj {
            println!("  Points to: {:?}", doc.arena().get_object(h));
        }
        return Ok(());
    }

    let page_idx = page_idx_str.parse::<usize>()? - 1;
    let page = doc.get_page(page_idx)?;
    println!("Page {} loaded.", page_idx + 1);
    
    for contents in page.contents_handles() {
        println!("Page Contents Handle: {:?}", contents);
        dump_resolved(&doc, Object::Reference(contents));
    }

    let res_h = page.resources_handle();
    let res_dict = doc.arena().get_dict(res_h).unwrap();
    
    if let Some(egs_obj) = res_dict.get(&doc.arena().name("ExtGState")) {
        if let Some(egs_dh) = egs_obj.resolve(doc.arena()).as_dict_handle() {
            let egs_dict = doc.arena().get_dict(egs_dh).unwrap();
            println!("ExtGState entries:");
            for (nk, val) in egs_dict {
                let name = doc.arena().get_name(nk.clone()).unwrap();
                println!("  /{} -> {:?}", name.as_str(), val);
                if let Some(h) = val.resolve(doc.arena()).as_dict_handle() {
                    let d = doc.arena().get_dict(h).unwrap();
                    println!("    Content:");
                    for (k2, v2) in d {
                        let n2 = doc.arena().get_name(k2.clone()).unwrap();
                        println!("      /{} -> {:?}", n2.as_str(), v2);
                    }
                }
            }
        }
    }

    if let Some(xobj_obj) = res_dict.get(&doc.arena().name("XObject")) {
        if let Some(xobj_dh) = xobj_obj.resolve(doc.arena()).as_dict_handle() {
            let xobj_dict = doc.arena().get_dict(xobj_dh).unwrap();
            println!("XObject entries:");
            for (nk, val) in xobj_dict {
                let name = doc.arena().get_name(nk.clone()).unwrap();
                println!("  /{} ->", name.as_str());
                dump_resolved(&doc, val.resolve(doc.arena()));
            }
        }
    }

    if let Some(font_obj) = res_dict.get(&doc.arena().name("Font")) {
        if let Some(font_dh) = font_obj.resolve(doc.arena()).as_dict_handle() {
            let font_dict = doc.arena().get_dict(font_dh).unwrap();
            println!("Font entries:");
            for (nk, val) in font_dict {
                let name = doc.arena().get_name(nk.clone()).unwrap();
                println!("  /{} -> {:?}", name.as_str(), val);
                if let Some(dh) = val.resolve(doc.arena()).as_dict_handle() {
                    let d = doc.arena().get_dict(dh).unwrap();
                    println!("    Dictionary:");
                    for (k2, v2) in d {
                        let n2 = doc.arena().get_name(k2.clone()).unwrap();
                        println!("      /{} -> {:?}", n2.as_str(), v2);
                    }
                }
            }
        }
    }
    
    Ok(())
}
