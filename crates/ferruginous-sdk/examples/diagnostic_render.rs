#![allow(missing_docs)]
use bytes::Bytes;
use ferruginous_sdk::PdfDocument;
use kurbo::BezPath;
use ferruginous_render::text::{SkrifaBridge, TextLayoutOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: diagnostic_render <input.pdf>");
        return Ok(());
    }
    let input = &args[1];
    let data = std::fs::read(input)?;
    let doc = PdfDocument::open(Bytes::from(data)).map_err(|e| format!("{e:?}"))?;
    
    render_pages(&doc)?;
    Ok(())
}

fn render_pages(doc: &PdfDocument) -> Result<(), Box<dyn std::error::Error>> {
    let summary = doc.get_summary().map_err(|e| format!("{e:?}"))?;
    println!("Document Summary: Pages={}", summary.page_count);

    for page_idx in 0..summary.page_count {
        println!("Rendering page {}...", page_idx + 1);
        let mut bridge = SkrifaBridge::new(None);
        let options = TextLayoutOptions::default();
        
        let mut path = BezPath::new();
        let glyphs = vec![(36, 600.0), (72, 600.0)]; 
        let p = bridge.render_glyphs(&[], &glyphs, &options);
        path.extend(p);
        
        println!("Page {} path elements: {}", page_idx + 1, path.segments().count());
    }
    Ok(())
}
