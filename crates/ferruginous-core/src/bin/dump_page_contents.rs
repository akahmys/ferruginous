use ferruginous_core::document::Document;
use ferruginous_core::object::Object;
use ferruginous_core::object::sublimation::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Dumping page contents...");
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: dump_page_contents <pdf_file> <page_num>");
        return Ok(());
    }

    println!("Loading document: {}...", args[1]);
    let doc = Document::load(std::path::Path::new(&args[1]))?;
    println!("Document loaded.");
    let page_num: usize = args[2].parse()?;
    println!("Retrieving page {}...", page_num);
    let page = doc.get_page(page_num - 1)?;
    println!("Page retrieved.");
    let page_dh = doc.resolve_to_dict(page.obj_handle())?;
    let page_dict = doc.arena().get_dict(page_dh).unwrap();
    println!("Page {}:", page_num);
    println!("Resolving Resources...");
    let _contents_key = doc.arena().name("Contents");
    if let Some(res_obj) = page.resolve_attribute("Resources") {
        let resolved = res_obj.resolve(doc.arena());
        println!("Resources found: {:?}", resolved);
        if let Some(res_dh) = resolved.as_dict_handle() {
            if let Some(res_dict) = doc.arena().get_dict(res_dh) {
                println!("  Resource keys:");
                for (name_h, _) in &res_dict {
                    println!("    /{}", doc.arena().get_name(*name_h).unwrap().as_str());
                }
                let font_key = doc.arena().name("Font");
                if let Some(f_obj) = res_dict.get(&font_key) {
                    if let Some(f_dh) = f_obj.resolve(doc.arena()).as_dict_handle() {
                        if let Some(f_dict) = doc.arena().get_dict(f_dh) {
                            println!("  Fonts in Resources:");
                            for (name_h, font_obj) in &f_dict {
                                let alias = doc.arena().get_name(*name_h).unwrap().as_str().to_string();
                                if let Some(h) = font_obj.as_reference() {
                                    println!("    {} -> Object {}", alias, h.index());
                                } else {
                                    println!("    {} -> {:?}", alias, font_obj);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    let contents_handles = page.contents_handles();

    println!("Page {}: (Found {} content streams)", page_num, contents_handles.len());
    for h in contents_handles {
        if let Some(sublimated) = doc.get_sublimated_data(h) {
            match &*sublimated {
                ferruginous_core::object::SublimatedData::Commands(cmds) => {
                    println!("  Found {} commands", cmds.len());
                    for cmd in cmds {
                        match cmd {
                            Command::SetFont { font, size } => {
                                println!("    Font: {} Size: {}", font, size);
                            }
                            Command::ShowText(bytes) => {
                                println!("    Text: {:?}", bytes);
                            }
                            Command::ShowTextArray(items) => {
                                print!("    TextArray: ");
                                for item in items {
                                    match item {
                                        ferruginous_core::object::sublimation::TextArrayItem::Text(b) => print!("{:?} ", b),
                                        ferruginous_core::object::sublimation::TextArrayItem::Offset(o) => print!("{} ", o),
                                    }
                                }
                                println!();
                            }
                            Command::BeginMarkedContent { tag, properties } => {
                                println!("    BeginMarkedContent: tag={}, props={:?}", tag.0, properties);
                            }
                            Command::EndMarkedContent => {
                                println!("    EndMarkedContent");
                            }
                            Command::DrawXObject(name) => {
                                println!("    DrawXObject: {}", name);
                            }
                            Command::Transform(m) => {
                                println!("    Transform: {:?}", m);
                            }
                            Command::PushState => {
                                println!("    PushState (q)");
                            }
                            Command::PopState => {
                                println!("    PopState (Q)");
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    println!("  Stream is NOT sublimated into Commands! State: {:?}", sublimated);
                }
            }
        } else {
            println!("  No sublimated data for handle {:?}", h);
        }
    }

    Ok(())
}
