use lopdf::Document;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: check_outline <input.pdf>");
        return Ok(());
    }

    let doc = Document::load(&args[1])?;
    
    // Root -> Outlines
    if let Ok(root) = doc.catalog()
        && let Ok(outlines_ref) = root.get(b"Outlines").and_then(|o| o.as_reference())
        && let Ok(outlines) = doc.get_object(outlines_ref)
        && let Ok(dict) = outlines.as_dict()
        && let Ok(first_ref) = dict.get(b"First").and_then(|o| o.as_reference()) {
        dump_outline_recursive(&doc, first_ref, 0)?;
    } else {
        println!("No Outlines found in Catalog.");
    }

    Ok(())
}

fn dump_outline_recursive(doc: &lopdf::Document, id: lopdf::ObjectId, depth: usize) -> anyhow::Result<()> {
    if depth > 2 { return Ok(()); } // 深さ制限
    
    if let Ok(obj) = doc.get_object(id)
        && let Ok(dict) = obj.as_dict()
        && let Ok(title_obj) = dict.get(b"Title") {
        match title_obj {
            lopdf::Object::String(s, _) => {
                println!("{}{} (String)", "  ".repeat(depth), String::from_utf8_lossy(s));
            }
            lopdf::Object::Stream(stream) => {
                println!("{}{} (Stream)", "  ".repeat(depth), String::from_utf8_lossy(&stream.content));
            }
            _ => println!("{}Untitled", "  ".repeat(depth)),
        }
        
        if let Ok(first_ref) = dict.get(b"First").and_then(|o| o.as_reference()) {
            dump_outline_recursive(doc, first_ref, depth + 1)?;
        }
        
        if let Ok(next_ref) = dict.get(b"Next").and_then(|o| o.as_reference()) {
            dump_outline_recursive(doc, next_ref, depth)?;
        }
    }
    Ok(())
}
