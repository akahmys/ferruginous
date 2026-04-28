#![allow(missing_docs)]
use ferruginous_core::{Document, Object};
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: stream_dump <pdf_file>");
        return Ok(());
    }

    let buf = fs::read(&args[1])?;
    let doc = Document::open(
        bytes::Bytes::from(buf),
        &ferruginous_core::ingest::IngestionOptions::default(),
    ).map_err(|e| format!("{e:?}"))?;
    let arena = doc.arena();

    for h in 0..arena.object_count() {
        let handle = ferruginous_core::handle::Handle::new(h);
        if let Some(obj) = arena.get_object(handle)
            && let Object::Stream(_, _) = obj
            && let Ok(decoded) = doc.decode_stream(&obj)
        {
            let content = String::from_utf8_lossy(&decoded);
            if content.contains("BT") {
                println!("--- Stream {h} ---");
                println!("{content}");
            }
        }
    }
    Ok(())
}
