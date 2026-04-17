#![allow(clippy::all, missing_docs)]
//! Verification utility for font analysis.

use ferruginous_sdk::core::types::Resolver;
use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::core::Object;
use ferruginous_sdk::font::Font;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let pdf_path = "samples/legacy/writing-mode-1.pdf";
    if !std::path::Path::new(pdf_path).exists() {
        println!("Skipping legacy verification: {} not found", pdf_path);
        return Ok(());
    }
    
    let data = std::fs::read(pdf_path)?;
    let doc = load_document_structure(&data)?;
    let resolver = doc.resolver();

    println!("--- Step 1.2 Verification for {pdf_path} ---");

    // Heuristic scan for font objects
    let vertical_font_found = scan_fonts(&resolver);

    if !vertical_font_found {
        println!("No vertical fonts found in the scanned range of this PDF.");
    }
    Ok(())
}

fn scan_fonts(resolver: &dyn Resolver) -> bool {
    let mut vertical_font_found = false;
    for i in 1..1000 {
        let r = ferruginous_sdk::core::Reference { id: i, generation: 0 };
        if let Ok(Object::Dictionary(dict)) = resolver.resolve(&r) {
            if let Some(Object::Name(subtype)) = dict.get(b"Subtype".as_ref()) {
                if subtype.as_ref() == b"Type0" {
                    process_type0_font(r, &dict, resolver, &mut vertical_font_found);
                }
            }
        }
    }
    vertical_font_found
}

fn process_type0_font(
    r: ferruginous_sdk::core::Reference, 
    dict: &std::collections::BTreeMap<Vec<u8>, Object>, 
    resolver: &dyn Resolver,
    vertical_font_found: &mut bool
) {
    println!("Found Type0 Font at {r:?}");
    if let Ok(font) = Font::from_dict(dict, resolver) {
        println!("  BaseFont: {}", String::from_utf8_lossy(&font.base_font));
        println!("  Computed info -> is_vertical: {}", font.is_vertical());
        
        if font.is_vertical() {
            *vertical_font_found = true;
            check_vertical_rotations(&font);
        }
    }
}

fn check_vertical_rotations(font: &Font) {
    let test_chars: Vec<&[u8]> = vec![b"A", b"1", b"(", b"\x30\x01"]; 
    for &c in &test_chars {
        let should_rotate = font.char_should_rotate_vertical(c);
        println!("  Should rotate {:?}: {}", String::from_utf8_lossy(c), should_rotate);
    }
}
