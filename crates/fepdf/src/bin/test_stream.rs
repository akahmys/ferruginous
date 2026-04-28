use ferruginous_core::{Document, PdfName, Object, Handle};
use std::fs;

fn main() {
    let data = fs::read("/Users/jun/Downloads/converted/bokutokitan.pdf").unwrap();
    let doc = Document::parse(&data).unwrap();
    let pages = doc.pages().unwrap();
    let page2 = doc.arena().get_dict(pages[1]).unwrap();
    let contents_key = doc.arena().intern_name(PdfName::new("Contents"));
    if let Some(contents) = page2.get(&contents_key).map(|o| o.resolve(doc.arena())) {
        let stream = match contents {
            Object::Stream(dh, stream_data) => Some((dh, stream_data)),
            Object::Array(arr_h) => {
                let arr = doc.arena().get_array(arr_h).unwrap();
                if let Object::Stream(dh, stream_data) = arr[0].resolve(doc.arena()) {
                    Some((dh, stream_data))
                } else { None }
            },
            _ => None,
        };
        if let Some((dh, stream_data)) = stream {
            let dict = doc.arena().get_dict(dh).unwrap();
            let decoded = doc.arena().process_filters(stream_data, dict).unwrap();
            println!("Content stream:\n{}", String::from_utf8_lossy(&decoded));
        } else {
            println!("Contents is not a stream");
        }
    } else {
        println!("No contents");
    }
}
