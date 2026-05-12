use ferruginous_core::document::Document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut page_num = 1;
    let mut file_path = String::new();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--page" && i + 1 < args.len() {
            page_num = args[i + 1].parse::<usize>()?;
            i += 2;
        } else if file_path.is_empty() {
            file_path = args[i].clone();
            i += 1;
        } else if i == args.len() - 1 && args[i].parse::<usize>().is_ok() {
            // Support legacy positional page num
            page_num = args[i].parse::<usize>()?;
            i += 1;
        } else {
            i += 1;
        }
    }

    if file_path.is_empty() {
        println!("Usage: inspect_text <pdf_file> [--page <page_num>]");
        return Ok(());
    }

    let doc = Document::load(std::path::Path::new(&file_path))?;
    let page = doc.get_page(page_num)?;

    println!("Content Stream for Page {}:", page_num);
    for contents_handle in page.contents_handles() {
        let contents_obj = doc.resolve(&contents_handle)?;
        let data = doc.decode_stream(&contents_obj)?;

        let mut i = 0;
        while i < data.len() {
            if data[i..].starts_with(b"(") || data[i..].starts_with(b"<") {
                let start = i;
                let mut end = i;
                if data[i] == b'(' {
                    let mut depth = 0;
                    for j in i..data.len() {
                        if data[j] == b'(' {
                            depth += 1;
                        } else if data[j] == b')' {
                            depth -= 1;
                        }
                        if depth == 0 {
                            end = j + 1;
                            break;
                        }
                    }
                } else {
                    for j in i..data.len() {
                        if data[j] == b'>' {
                            end = j + 1;
                            break;
                        }
                    }
                }

                let next_chunk = &data[end..end + 10.min(data.len() - end)];
                if next_chunk.contains(&b'T')
                    && (next_chunk.contains(&b'j') || next_chunk.contains(&b'J'))
                {
                    println!("Text: {:?}", &data[start..end]);
                }
                i = end;
            } else {
                i += 1;
            }
        }
    }

    Ok(())
}
