#![allow(missing_docs)]
use ferruginous_sdk::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read("/Users/jun/Downloads/nihonkokukenpou.pdf")?;
    let doc = PdfDocument::open(data.into()).map_err(|e| format!("{e:?}"))?;
    let page = doc.inner().get_page(4).map_err(|e| format!("{e:?}"))?;
    let contents = page.resolve_attribute("Contents").ok_or("No contents")?;
    let stream = contents.resolve(doc.inner().arena());
    let decoded = doc.inner().decode_stream(&stream).map_err(|e| format!("{e:?}"))?;
    println!("{}", String::from_utf8_lossy(&decoded));
    Ok(())
}
