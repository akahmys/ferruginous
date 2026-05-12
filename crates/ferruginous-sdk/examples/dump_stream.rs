use ferruginous_core::{Object, PdfName};
use ferruginous_sdk::PdfDocument;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path_str = args.get(1).map(|s| s.as_str()).unwrap_or("samples/bokutokitan.pdf");

    let data = std::fs::read(path_str)?;
    let doc = PdfDocument::open(data.into())?;

    let page = doc.get_page(0)?;
    println!("Page 0 media box: {:?}", page.media_box());

    let arena = doc.inner().arena();
    for contents_h in page.contents_handles() {
        let stream = arena.get_stream_bytes(contents_h).unwrap();
        let decoded = doc.inner().decode_stream(stream).unwrap();
        println!("Stream length: {}", decoded.len());
        // println!("{}", String::from_utf8_lossy(&decoded));
    }

    Ok(())
}
