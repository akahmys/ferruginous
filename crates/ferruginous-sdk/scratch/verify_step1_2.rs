use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::core::Object;
use ferruginous_sdk::font::Font;
use std::path::Path;

fn main() {
    let pdf_path = "../../samples/legacy/writing-mode-1.pdf";
    let data = std::fs::read(pdf_path).expect("failed to read pdf");
    let doc = load_document_structure(&data).expect("failed to load doc");
    let resolver = doc.resolver();

    println!("--- Step 1.2 Verification for {} ---", pdf_path);

    // Heuristic scan for font objects
    let vertical_font_found = scan_fonts(resolver);

    if !vertical_font_found {
        println!("No vertical fonts found in the scanned range of this PDF.");
    }
}

fn scan_fonts(resolver: &dyn ferruginous_sdk::core::Resolver) -> bool {
    let mut vertical_font_found = false;
    for i in 1..1000 {
        let r = ferruginous_sdk::core::Reference { id: i, gen: 0 };
        if let Ok(Object::Dictionary(dict)) = resolver.resolve(&r) {
            if let Some(Object::Name(subtype)) = dict.get(b"Subtype".as_ref()) {
                if subtype.as_slice() == b"Type0" {
                    println!("Found Type0 Font at {:?}", r);
                    if let Ok(font) = Font::from_dict(&dict, resolver) {
                        println!("  BaseFont: {}", String::from_utf8_lossy(&font.base_font));
                        println!("  is_vertical: {}", font.is_vertical());
                        
                        if font.is_vertical() {
                            vertical_font_found = true;
                            check_vertical_rotations(&font);
                        }
                    }
                }
            }
        }
    }
    vertical_font_found
}

fn check_vertical_rotations(font: &Font) {
    let test_chars = [b"A", b"1", b"(", b"\x30\x01"]; 
    for &c in &test_chars {
        let should_rotate = font.char_should_rotate_vertical(c);
        println!("  Should rotate {:?}: {}", String::from_utf8_lossy(c), should_rotate);
    }
}
