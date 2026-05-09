use ferruginous_core::arena::PdfArena;
use ferruginous_core::font::FontResource;
use ferruginous_core::object::Object;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arena = PdfArena::new();
    let data = std::fs::read("samples/volvo_xc90.pdf")?;
    let doc = arena.load_document(&data)?;
    
    // Find page 50
    let pages = doc.pages();
    let page_50 = &pages[49];
    
    println!("--- Fonts on Page 50 ---");
    let resources = page_50.resources();
    if let Some(fonts) = resources.get("Font") {
        if let Object::Dictionary(dict) = fonts {
            for (name, font_ref) in dict {
                println!("Font: {}", name);
                if let Ok(font_res) = FontResource::load(font_ref, &arena) {
                    println!("  BaseFont: {:?}", font_res.base_font);
                    println!("  Subtype: {}", font_res.subtype);
                    if let Some(ref tu) = font_res.to_unicode {
                        println!("  ToUnicode Mappings: {} entries", tu.mappings.len());
                        for (code, unicode) in tu.mappings.iter().take(20) {
                            println!("    {:02x?} -> {:?}", code, unicode);
                        }
                    } else {
                        println!("  No ToUnicode");
                    }
                    println!("  CIDToGIDMap: {}", if font_res.cid_to_gid_map.is_some() { "Present" } else { "None" });
                }
            }
        }
    }
    
    Ok(())
}
