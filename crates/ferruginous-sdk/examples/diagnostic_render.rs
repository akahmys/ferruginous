#![allow(missing_docs)]
use ferruginous_sdk::PdfDocument;
use bytes::Bytes;
use std::path::Path;

fn main() {
    let files = [
        "/Users/jun/Downloads/nihonkokukenpou.pdf",
        "/Users/jun/Downloads/bokutokitan.pdf"
    ];

    for file_path in files {
        println!("\n=== Analyzing: {file_path} ===");
        let path = Path::new(file_path);
        if !path.exists() {
            println!("  File not found!");
            continue;
        }
        let data = std::fs::read(path).unwrap();
        let doc = PdfDocument::open(Bytes::from(data)).unwrap();
        
        let p_idx = if file_path.contains("nihon") { 4 } else { 2 }; // Page 5 and 3
        let page = doc.inner().get_page(p_idx).unwrap();
        
        if let Some(media_box) = page.resolve_attribute("MediaBox") {
            println!("  MediaBox: {:?}", media_box.resolve(doc.inner().arena()));
        }
        
        if let Some(res) = page.resolve_attribute("Resources") {
            let res = res.resolve(doc.inner().arena());
            if let Some(dict_h) = res.as_dict_handle() {
                let dict = doc.inner().arena().get_dict(dict_h).unwrap();
                let font_key = doc.inner().arena().name("Font");
                if let Some(fonts_obj) = dict.get(&font_key) {
                    let fonts_res = fonts_obj.resolve(doc.inner().arena());
                    if let Some(fonts) = fonts_res.as_dict_handle() {
                        let fonts_dict = doc.inner().arena().get_dict(fonts).unwrap();
                        for (name_h, font_obj) in fonts_dict {
                            let name = doc.inner().arena().get_name(name_h).unwrap();
                            let f_dict_obj = font_obj.resolve(doc.inner().arena());
                            println!("  Font {}: {:?}", name.as_str(), f_dict_obj);
                            
                            if let Some(f_dict) = f_dict_obj.as_dict_handle().and_then(|h| doc.inner().arena().get_dict(h)) {
                                let to_uni_key = doc.inner().arena().name("ToUnicode");
                                if let Some(to_uni) = f_dict.get(&to_uni_key) {
                                    let stream = to_uni.resolve(doc.inner().arena());
                                    if let Ok(data) = doc.inner().decode_stream(&stream) {
                                        println!("    ToUnicode CMap: {} bytes", data.len());
                                        println!("    --- [ CMAP START ] ---");
                                        println!("{}", String::from_utf8_lossy(&data));
                                        println!("    --- [ CMAP END ] ---");
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
