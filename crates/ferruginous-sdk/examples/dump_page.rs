#![allow(missing_docs)]
use bytes::Bytes;
use ferruginous_sdk::PdfDocument;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("/Users/jun/Downloads/bokutokitan.pdf");
    if !path.exists() {
        eprintln!("File not found: {}", path.display());
        return Ok(());
    }
    let data = std::fs::read(path)?;
    let doc = PdfDocument::open(Bytes::from(data)).map_err(|e| format!("{e:?}"))?;

    let page_count = doc.page_count().map_err(|e| format!("{e:?}"))?;
    if page_count < 3 {
        eprintln!("Document only has {page_count} pages");
        return Ok(());
    }

    let summary = doc.get_summary().map_err(|e| format!("{e:?}"))?;
    println!("Document Version: {}", summary.version);

    let page = doc.inner().get_page(2).map_err(|e| format!("{e:?}"))?;
    if let Some(contents_obj) = page.resolve_attribute("Contents") {
        let actual_obj = contents_obj.resolve(doc.inner().arena());
        match actual_obj {
            ferruginous_core::Object::Array(h) => {
                if let Some(array) = doc.inner().arena().get_array(h) {
                    println!("--- [ PAGE 3 CONTENT STREAMS (ARRAY OF {}) ] ---", array.len());
                    for (i, obj) in array.iter().enumerate() {
                        let stream_obj = obj.resolve(doc.inner().arena());
                        if let Ok(data) = doc.inner().decode_stream(&stream_obj) {
                            println!("--- [ STREAM {i} ] ---");
                            println!("{}", String::from_utf8_lossy(&data));
                        }
                    }
                }
            }
            _ => {
                if let Ok(data) = doc.inner().decode_stream(&actual_obj) {
                    println!("--- [ PAGE 3 CONTENT STREAM ] ---");
                    println!("{}", String::from_utf8_lossy(&data));
                }
            }
        }
    }
    Ok(())
}
