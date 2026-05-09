use ferruginous_core::object::sublimation::{Command, TextArrayItem};
use ferruginous_sdk::PdfDocument;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path_str = args.get(1).map(|s| s.as_str()).unwrap_or("samples/bokutokitan.pdf");

    println!("Checking {}...", path_str);
    let data = std::fs::read(path_str)?;
    let doc = PdfDocument::open(data.into())?;
    doc.inner().arena().sublimate_all()?;

    if doc.page_count()? > 0 {
        let page = doc.get_page(0)?;
        let handles = page.contents_handles();

        for h in handles {
            if let Some(sublimated) = doc.inner().arena().get_sublimated_data(h.clone()) {
                if let ferruginous_core::object::SublimatedData::Commands(cmds) = &*sublimated {
                    for cmd in cmds {
                        match cmd {
                            Command::ShowText(bytes) => {
                                println!("Tj: {:?}", bytes);
                            }
                            Command::ShowTextArray(items) => {
                                print!("TJ: [");
                                for item in items {
                                    match item {
                                        TextArrayItem::Text(b) => print!("{:?}, ", b),
                                        TextArrayItem::Offset(o) => print!("{}, ", o),
                                    }
                                }
                                println!("]");
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
