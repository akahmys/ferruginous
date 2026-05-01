use ferruginous_core::security::SecurityHandler;
use lopdf::Document;
use std::collections::BTreeMap;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: bypass_decrypt <input.pdf> <output.pdf>");
        return Ok(());
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Bypassing Arena: Loading {:?}...", input_path);
    let mut doc = Document::load(input_path)?;

    println!("Applying Pass 0 Decryption...");
    perform_pass_0_decryption_debug(&mut doc)?;

    println!("Saving to {:?}...", output_path);
    doc.save(output_path)?;

    println!("SUCCESS: Decrypted PDF saved to {:?}", output_path);
    Ok(())
}

fn perform_pass_0_decryption_debug(doc: &mut lopdf::Document) -> anyhow::Result<()> {
    count_object_types(doc);

    if let Some(obj8253) = doc.objects.get(&(8253, 0)) {
        println!(
            "Object 8253 type: {:?}",
            match obj8253 {
                lopdf::Object::String(_, _) => "String",
                lopdf::Object::Array(_) => "Array",
                lopdf::Object::Dictionary(_) => "Dictionary",
                lopdf::Object::Stream(_) => "Stream",
                _ => "Other",
            }
        );
    }

    if let Some(handler) = init_security_handler(doc) {
        let mut string_count = 0;
        let mut stream_count = 0;
        let mut fail_count = 0;

        let ids: Vec<lopdf::ObjectId> = doc.objects.keys().cloned().collect();

        for id in ids {
            if let Some(obj) = doc.objects.get_mut(&id) {
                decrypt_object_recursive(
                    obj,
                    id,
                    &handler,
                    &mut string_count,
                    &mut stream_count,
                    &mut fail_count,
                );
            }
        }
        println!(
            "Decryption stats: Strings={}, Streams={}, Fails={}",
            string_count, stream_count, fail_count
        );
        doc.trailer.remove(b"Encrypt");
        println!("Removed /Encrypt from trailer.");
    }
    Ok(())
}

fn count_object_types(doc: &lopdf::Document) {
    let mut type_counts = BTreeMap::new();
    for obj in doc.objects.values() {
        let type_name = match obj {
            lopdf::Object::Null => "Null",
            lopdf::Object::Boolean(_) => "Boolean",
            lopdf::Object::Integer(_) => "Integer",
            lopdf::Object::Real(_) => "Real",
            lopdf::Object::Name(_) => "Name",
            lopdf::Object::String(_, _) => "String",
            lopdf::Object::Array(_) => "Array",
            lopdf::Object::Dictionary(_) => "Dictionary",
            lopdf::Object::Stream(_) => "Stream",
            lopdf::Object::Reference(_) => "Reference",
        };
        *type_counts.entry(type_name).or_insert(0) += 1;
    }
    println!("Object type counts: {:?}", type_counts);
}

fn init_security_handler(doc: &lopdf::Document) -> Option<SecurityHandler> {
    let encrypt_dict_obj = doc.trailer.get(b"Encrypt").ok()?;
    println!("Found Encrypt entry in trailer.");

    let encrypt_obj = if let Ok(id) = encrypt_dict_obj.as_reference() {
        println!("Encrypt dict is at reference {:?}", id);
        doc.objects.get(&id)
    } else {
        Some(encrypt_dict_obj)
    };

    let lopdf::Object::Dictionary(dict) = encrypt_obj? else {
        println!("Encrypt entry found but not a dictionary.");
        return None;
    };

    let (v_val, r_val) = (
        dict.get(b"V").and_then(|o| o.as_i64()).unwrap_or(0),
        dict.get(b"R").and_then(|o| o.as_i64()).unwrap_or(0),
    );
    println!("V: {}, R: {}", v_val, r_val);

    if (v_val == 4 && r_val == 4) || (v_val == 1 && r_val == 3) || (v_val == 2 && r_val == 3) {
        init_v4_handler(doc, dict)
    } else if v_val == 5 && r_val == 5 {
        init_v5_handler(doc)
    } else {
        println!("Unsupported Encryption Version/Revision: V={}, R={}", v_val, r_val);
        None
    }
}

fn init_v4_handler(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> Option<SecurityHandler> {
    let o_str = dict.get(b"O").and_then(|o| o.as_str()).unwrap_or(&[]);
    let u_str = dict.get(b"U").and_then(|o| o.as_str()).unwrap_or(&[]);
    let p_val = dict.get(b"P").and_then(|o| o.as_i64()).unwrap_or(0) as i32;

    let mut file_id = &[][..];
    if let Ok(id_obj) = doc.trailer.get(b"ID")
        && let Ok(arr) = id_obj.as_array()
        && let Some(first) = arr.first().and_then(|o| o.as_str().ok())
    {
        file_id = first;
    }

    println!(
        "O len: {}, U len: {}, P: {}, ID len: {}",
        o_str.len(),
        u_str.len(),
        p_val,
        file_id.len()
    );

    match SecurityHandler::new_v4("", o_str, u_str, p_val, file_id, true) {
        Ok(handler) => {
            println!("SecurityHandler V4 initialized successfully.");
            Some(handler)
        }
        Err(e) => {
            println!("Failed to initialize SecurityHandler V4: {:?}", e);
            None
        }
    }
}

fn init_v5_handler(doc: &lopdf::Document) -> Option<SecurityHandler> {
    let mut file_id = &[][..];
    if let Ok(id_array) = doc.trailer.get(b"ID")
        && let Ok(arr) = id_array.as_array()
        && let Some(first) = arr.first().and_then(|o| o.as_str().ok())
    {
        file_id = first;
    }
    match SecurityHandler::new_v5("", "", file_id) {
        Ok(handler) => {
            println!("SecurityHandler V5 initialized successfully.");
            Some(handler)
        }
        Err(e) => {
            println!("Failed to initialize SecurityHandler V5: {:?}", e);
            None
        }
    }
}

fn decrypt_object_recursive(
    obj: &mut lopdf::Object,
    id: lopdf::ObjectId,
    handler: &SecurityHandler,
    string_count: &mut usize,
    stream_count: &mut usize,
    fail_count: &mut usize,
) {
    match obj {
        lopdf::Object::String(s, _) => {
            handle_decrypt_string(s, id, handler, string_count, fail_count)
        }
        lopdf::Object::Stream(stream) => {
            handle_decrypt_stream(stream, id, handler, stream_count, fail_count)
        }
        lopdf::Object::Array(arr) => {
            for item in arr {
                decrypt_object_recursive(item, id, handler, string_count, stream_count, fail_count);
            }
        }
        lopdf::Object::Dictionary(dict) => {
            for (_, item) in dict.iter_mut() {
                decrypt_object_recursive(item, id, handler, string_count, stream_count, fail_count);
            }
        }
        _ => {}
    }
}

fn handle_decrypt_string(
    s: &mut Vec<u8>,
    id: lopdf::ObjectId,
    handler: &SecurityHandler,
    count: &mut usize,
    fail: &mut usize,
) {
    *count += 1;
    match handler.decrypt_bytes(s, id.0, id.1) {
        Ok(decrypted) => *s = decrypted,
        Err(_) => *fail += 1,
    }
}

fn handle_decrypt_stream(
    stream: &mut lopdf::Stream,
    id: lopdf::ObjectId,
    handler: &SecurityHandler,
    count: &mut usize,
    fail: &mut usize,
) {
    if let Ok(type_obj) = stream.dict.get(b"Type")
        && let Ok(name) = type_obj.as_name_str()
        && (name == "ObjStm" || name == "XRef")
    {
        return;
    }
    *count += 1;
    match handler.decrypt_bytes(&stream.content, id.0, id.1) {
        Ok(decrypted) => {
            if *count < 5 {
                println!(
                    "DEBUG: Stream {:?} decrypted sample: {:?}",
                    id,
                    &decrypted[..decrypted.len().min(16)]
                );
            }
            stream.content = decrypted;
        }
        Err(_) => *fail += 1,
    }
}
