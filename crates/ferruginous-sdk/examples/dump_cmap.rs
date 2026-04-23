#![allow(missing_docs)]
use ferruginous_sdk::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read("/Users/jun/Downloads/nihonkokukenpou.pdf")?;
    let doc = PdfDocument::open(data.into())?;
    let arena = doc.inner().arena();
    let fonts_node = doc.inner().get_page(4)?.resolve_attribute("Resources").expect("No res")
        .resolve(arena).as_dict_handle().expect("No dict");
    let fonts = arena.get_dict(fonts_node).expect("No dict")
        .get(&arena.name("Font")).expect("No font cat").resolve(arena).as_dict_handle().expect("No font dict");
    let fonts_dict = arena.get_dict(fonts).expect("No font dict");
    
    for (name_h, font_obj) in fonts_dict {
        let name = arena.get_name(name_h).unwrap();
        println!("--- Font: {} ---", name.as_str());
        let f_dict_obj = font_obj.resolve(arena);
        if let Some(f_dict_h) = f_dict_obj.as_dict_handle() {
            let fd = arena.get_dict(f_dict_h).expect("No font dict");
            if let Some(to_uni) = fd.get(&arena.name("ToUnicode")) {
                let stream = to_uni.resolve(arena);
                if let Ok(data) = doc.inner().decode_stream(&stream) {
                    println!("ToUnicode CMap:\n{}", String::from_utf8_lossy(&data));
                }
            } else {
                println!("No ToUnicode CMap");
            }
        }
    }
    Ok(())
}
