use ferruginous_core::{Document, PdfName, Object};
use std::fs;

fn main() {
    let data = fs::read("/Users/jun/Downloads/converted/bokutokitan.pdf").unwrap();
    let doc = Document::parse(&data).unwrap();
    let pages = doc.pages().unwrap();
    let page = doc.arena().get_dict(pages[2]).unwrap(); // Page 3 is index 2
    let contents_key = doc.arena().intern_name(PdfName::new("Contents"));
    if let Some(contents) = page.get(&contents_key).map(|o| doc.arena().resolve(o)) {
        let stream = match contents {
            Object::Stream(dh, stream_data) => Some((dh, stream_data)),
            Object::Array(arr_h) => {
                let arr = doc.arena().get_array(arr_h).unwrap();
                if let Object::Stream(dh, stream_data) = doc.arena().resolve(&arr[0]) {
                    Some((dh, stream_data))
                } else { None }
            },
            _ => None,
        };
        if let Some((dh, stream_data)) = stream {
            let dict = doc.arena().get_dict(dh).unwrap();
            let decoded = doc.arena().process_filters(stream_data.clone(), dict).unwrap();
            let content = String::from_utf8_lossy(&decoded);
            println!("--- Page 3 Content Stream ---");
            for line in content.lines().take(150) {
                println!("{}", line);
            }
        }
    }
}
