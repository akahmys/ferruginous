#![allow(missing_docs)]
use ferruginous_sdk::PdfDocument;
use bytes::Bytes;
use std::path::Path;

fn main() {
    let path = Path::new("/Users/jun/Downloads/bokutokitan.pdf");
    if !path.exists() {
        eprintln!("File not found: {}", path.display());
        return;
    }
    let data = std::fs::read(path).unwrap();
    let doc = PdfDocument::open(Bytes::from(data)).unwrap();
    
    // Page 3 is index 2
    let page_count = doc.page_count().unwrap();
    if page_count < 3 {
        eprintln!("Document only has {page_count} pages");
        return;
    }
    
    let summary = doc.get_summary().unwrap();
    println!("Document Version: {}", summary.version);
    
    // Get contents via resolve_attribute
    let page = doc.inner().get_page(2).unwrap();
    if let Some(contents_obj) = page.resolve_attribute("Contents") {
        let actual_obj = contents_obj.resolve(doc.inner().arena());
        match actual_obj {
            ferruginous_core::Object::Array(h) => {
                let array = doc.inner().arena().get_array(h).unwrap();
                println!("--- [ PAGE 3 CONTENT STREAMS (ARRAY OF {}) ] ---", array.len());
                for (i, obj) in array.iter().enumerate() {
                    let stream_obj = obj.resolve(doc.inner().arena());
                    let data = doc.inner().decode_stream(&stream_obj).unwrap();
                    println!("--- [ STREAM {i} ] ---");
                    println!("{}", String::from_utf8_lossy(&data));
                }
            }
            _ => {
                let data = doc.inner().decode_stream(&actual_obj).unwrap();
                println!("--- [ PAGE 3 CONTENT STREAM ] ---");
                println!("{}", String::from_utf8_lossy(&data));
            }
        }
    } else {
        println!("Page 3 has no contents.");
    }
}
