use ttf_parser;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: font_check <font_file>");
        return;
    }
    let data = fs::read(&args[1]).unwrap();
    if let Ok(face) = ttf_parser::Face::parse(&data, 0) {
        println!("Glyph count: {}", face.number_of_glyphs());
        if let Some(cmap) = face.tables().cmap {
            for subtable in cmap.subtables {
                println!("Cmap subtable: platform={:?}, encoding={:?}, format={:?}", subtable.platform_id, subtable.encoding_id, subtable.format);
            }
        }
    } else {
        println!("Failed to parse font");
    }
}
