#![allow(missing_docs)]
use ferruginous_sdk::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read("/Users/jun/Downloads/nihonkokukenpou.pdf")?;
    let doc = PdfDocument::open(data.into())?;
    let page = doc.inner().get_page(4)?; // Page 5
    let stream = page.resolve_attribute("Contents").expect("No contents");
    let decoded = doc.inner().decode_stream(&stream)?;
    println!("{}", String::from_utf8_lossy(&decoded));
    Ok(())
}
