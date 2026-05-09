use lopdf::Document;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let doc = Document::load(&args[1])?;
    let pages = doc.get_pages();
    if let Some((page_id, _)) = pages.get(&50) {
        println!("Page 50: {:?}", page_id);
        let page_dict = doc.get_object((*page_id, 0))?.as_dict()?;
        if let Ok(res_id) = page_dict.get(b"Resources") {
            let res = match res_id {
                lopdf::Object::Reference(r) => doc.get_object(*r)?.as_dict()?,
                lopdf::Object::Dictionary(d) => d,
                _ => return Err("Resources not a dict".into()),
            };
            if let Ok(fonts) = res.get(b"Font") {
                let fonts_dict = match fonts {
                    lopdf::Object::Reference(r) => doc.get_object(*r)?.as_dict()?,
                    lopdf::Object::Dictionary(d) => d,
                    _ => return Err("Font not a dict".into()),
                };
                for (name, id) in fonts_dict {
                    println!("Font {}: {:?}", String::from_utf8_lossy(name), id);
                }
            }
        }
    }
    Ok(())
}
