use ferruginous_core::document::Document;
use ferruginous_core::handle::Handle;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("samples/fy05.pdf");
    let doc = Document::load(path)?;
    let arena = doc.arena();

    let h34 = Handle::new(34);
    if let Some(name) = arena.get_name(h34) {
        println!("Handle 34 is: {}", name.as_str());
    }

    Ok(())
}
