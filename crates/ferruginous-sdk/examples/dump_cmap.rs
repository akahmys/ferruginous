#![allow(missing_docs)]
use ferruginous_sdk::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read("/Users/jun/Downloads/nihonkokukenpou.pdf")?;
    let doc = PdfDocument::open(data.into()).map_err(|e| format!("{e:?}"))?;
    let arena = doc.inner().arena();
    
    let page = doc.inner().get_page(4).map_err(|e| format!("{e:?}"))?;
    let res_obj = page.resolve_attribute("Resources").ok_or("No Resources")?.resolve(arena);
    let res_dict_h = res_obj.as_dict_handle().ok_or("Resources not a dict")?;
    let res_dict = arena.get_dict(res_dict_h).ok_or("Resources dict not found")?;
    
    let font_key = arena.name("Font");
    let fonts_obj = res_dict.get(&font_key).ok_or("No Font entry")?.resolve(arena);
    let fonts_h = fonts_obj.as_dict_handle().ok_or("Font entry not a dict")?;
    let fonts_dict = arena.get_dict(fonts_h).ok_or("Font dict not found")?;

    for (name_h, font_obj) in fonts_dict {
        let name = arena.get_name(name_h).ok_or("Font name handle invalid")?;
        println!("--- Font: {} ---", name.as_str());
        let f_dict_obj = font_obj.resolve(arena);
        if let Some(f_dict_h) = f_dict_obj.as_dict_handle()
            && let Some(fd) = arena.get_dict(f_dict_h)
            && let Some(to_uni) = fd.get(&arena.name("ToUnicode")) {
                let stream = to_uni.resolve(arena);
                if let Ok(data) = doc.inner().decode_stream(&stream) {
                    println!("ToUnicode CMap:\n{}", String::from_utf8_lossy(&data));
                }
        }
    }
    Ok(())
}
