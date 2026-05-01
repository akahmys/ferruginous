use ferruginous_core::Object;
use ferruginous_sdk::PdfDocument;
use std::fs;

fn main() {
    let data = fs::read("samples/nihonkokukenpou.pdf").expect("Failed to read sample PDF");
    let doc = PdfDocument::open(bytes::Bytes::from(data)).unwrap();
    let page_count = doc.page_count().unwrap();

    if page_count < 1 {
        println!("Document has no pages");
        return;
    }

    let page = doc.inner().get_page(0).unwrap();
    let arena = doc.inner().arena();

    if let Some(contents) = page.resolve_attribute("Contents") {
        let stream = match contents {
            Object::Stream(dh, stream_data) => Some((dh, stream_data)),
            Object::Array(arr_h) => {
                let arr = arena.get_array(arr_h).unwrap();
                if let Object::Stream(dh, stream_data) = arr[0].resolve(arena) {
                    Some((dh, stream_data))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some((dh, stream_data)) = stream {
            let dict = arena.get_dict(dh).unwrap();
            let raw_bytes = arena.get_stream_bytes(&stream_data).unwrap();
            let decoded = arena.process_filters(&raw_bytes, &dict).unwrap();
            println!("Content stream:\n{}", String::from_utf8_lossy(&decoded));
        } else {
            println!("Contents is not a stream");
        }
    } else {
        println!("No contents");
    }
}
