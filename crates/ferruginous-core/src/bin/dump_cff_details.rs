use ferruginous_core::document::Document;
use ferruginous_core::font::FontReconstructor;
use ferruginous_core::font::cff_standard::CFF_STANDARD_STRINGS;
use ferruginous_core::object::Object;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: dump_cff_details <pdf_file>");
        return Ok(());
    }

    let doc = Document::load(std::path::Path::new(&args[1]))?;
    let arena = doc.arena();

    let obj_id: Option<u32> = args.get(2).and_then(|s| s.parse().ok());

    let start = obj_id.unwrap_or(0) as usize;
    let end = if obj_id.is_some() { start + 1 } else { arena.object_count() as usize };

    for i in start..end {
        let handle = ferruginous_core::handle::Handle::new(i as u32);
        if let Some(Object::Dictionary(dh)) = arena.get_object(handle) {
            let dict = arena.get_dict(dh).unwrap();
            let base_font = dict
                .get(&arena.name("BaseFont"))
                .and_then(|o| o.resolve(arena).as_name())
                .and_then(|n| arena.get_name(n))
                .map(|n| n.as_str().to_string());

            if let Some(ref name) = base_font {
                println!("Font Object {}: {}", i, name);
                if let Some(fd_obj) = dict.get(&arena.name("FontDescriptor"))
                    && let Object::Dictionary(fd_h) = fd_obj.resolve(arena)
                {
                    let fd_dict = arena.get_dict(fd_h).unwrap();
                    if let Some(ff3) = fd_dict.get(&arena.name("FontFile3"))
                        && let Object::Stream(_, data) = ff3.resolve(arena)
                    {
                        println!("  CFF Stream found.");
                        let raw_data = arena.get_stream_bytes(&data)?;
                        let info = FontReconstructor::inspect_cff(&raw_data)?;
                        println!("  Is CID: {}", info.is_cid);
                        if let Some(ref map) = info.name_to_gid {
                            println!("  Name Map (Name -> GID [SID]):");
                            // Build a reverse map from SID to GID for easier display if needed,
                            // but here we want Name -> GID [SID].
                            // We need to re-scan sid_map to find the SID for each GID.
                            let mut gid_to_sid = std::collections::BTreeMap::new();
                            if let Some(ref sm) = info.sid_to_gid {
                                for (&sid, &gid) in sm {
                                    gid_to_sid.insert(gid, sid);
                                }
                            }

                            let mut sorted_map: Vec<_> = map.iter().collect();
                            sorted_map.sort_by_key(|&(_, gid)| *gid);
                            for (name, gid) in sorted_map {
                                let sid_str = gid_to_sid
                                    .get(gid)
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| "Unknown".to_string());
                                println!("    {} -> GID {} [SID {}]", name, gid, sid_str);
                            }
                        } else if let Some(ref map) = info.sid_to_gid {
                            println!("  Charset Map (GID -> SID):");
                            let mut gid_to_sid: std::collections::BTreeMap<u32, u32> =
                                std::collections::BTreeMap::new();
                            for (&sid, &gid) in map.iter() {
                                gid_to_sid.insert(gid, sid);
                            }
                            for (gid, sid) in gid_to_sid {
                                if info.is_cid {
                                    println!("    GID {} -> CID {}", gid, sid);
                                } else {
                                    let name = if (sid as usize) < CFF_STANDARD_STRINGS.len() {
                                        CFF_STANDARD_STRINGS[sid as usize].to_string()
                                    } else {
                                        // Try to resolve custom string if it was parsed
                                        info.string_index
                                            .get((sid as usize) - CFF_STANDARD_STRINGS.len())
                                            .cloned()
                                            .unwrap_or_else(|| "CustomString".to_string())
                                    };
                                    println!("    GID {} -> SID {} ({})", gid, sid, name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
