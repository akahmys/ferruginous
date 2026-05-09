use ferruginous_core::font::cmap::CMap;

fn main() {
    let cmap = CMap::adobe_japan1_ucs2();
    for cid in 1..100 {
        let code = vec![(cid >> 8) as u8, (cid & 0xFF) as u8];
        if let Some(s) = cmap.map(&code) {
            println!("CID {}: {} (U+{:04X})", cid, s, s.chars().next().unwrap() as u32);
        }
    }
}
