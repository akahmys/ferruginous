#![allow(missing_docs)]
use ferruginous_core::{Document, Object};
use std::fs;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: stream_dump <pdf_file>");
        return;
    }

    let buf = fs::read(&args[1]).expect("Failed to read file");
    let doc = Document::open(bytes::Bytes::from(buf), &ferruginous_core::ingest::IngestionOptions::default()).expect("Failed to open PDF");
    let arena = doc.arena();

    for h in 0..arena.object_count() {
        let handle = ferruginous_core::handle::Handle::new(u32::try_from(h).expect("Too many objects"));
        if let Some(obj) = arena.get_object(handle)
            && let Object::Stream(_, _) = obj
            && let Ok(decoded) = doc.decode_stream(&obj) {
                let content = String::from_utf8_lossy(&decoded);
                if content.contains("BT") {
                    println!("--- Stream {h} ---");
                    println!("{content}");
                }
        }
    }
}
