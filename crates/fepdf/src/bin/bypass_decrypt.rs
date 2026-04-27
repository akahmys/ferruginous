use lopdf::Document;
use ferruginous_core::security::SecurityHandler;
use std::env;
use std::collections::HashMap;

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
    let mut type_counts = HashMap::new();
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

    if let Some(obj8253) = doc.objects.get(&(8253, 0)) {
        println!("Object 8253 type: {:?}", match obj8253 {
            lopdf::Object::String(_, _) => "String",
            lopdf::Object::Array(_) => "Array",
            lopdf::Object::Dictionary(_) => "Dictionary",
            lopdf::Object::Stream(_) => "Stream",
            _ => "Other",
        });
    }

    let mut security_handler = None;
    
    if let Ok(encrypt_dict_obj) = doc.trailer.get(b"Encrypt") {
        println!("Found Encrypt entry in trailer.");
        let encrypt_obj = if let Ok(id) = encrypt_dict_obj.as_reference() {
            println!("Encrypt dict is at reference {:?}", id);
            doc.objects.get(&id)
        } else {
            Some(encrypt_dict_obj)
        };

        if let Some(lopdf::Object::Dictionary(dict)) = encrypt_obj {
            let v_val = dict.get(b"V").and_then(|o| o.as_i64()).unwrap_or(0);
            let r_val = dict.get(b"R").and_then(|o| o.as_i64()).unwrap_or(0);
            println!("V: {}, R: {}", v_val, r_val);
            
            if (v_val == 4 && r_val == 4) || (v_val == 1 && r_val == 3) || (v_val == 2 && r_val == 3) {
                let o_str = dict.get(b"O").and_then(|o| o.as_str()).unwrap_or(&[]);
                let u_str = dict.get(b"U").and_then(|o| o.as_str()).unwrap_or(&[]);
                let p_val = dict.get(b"P").and_then(|o| o.as_i64()).unwrap_or(0) as i32;
                
                let mut file_id = &[][..];
                if let Ok(id_obj) = doc.trailer.get(b"ID")
                    && let Ok(arr) = id_obj.as_array()
                    && let Some(first) = arr.first().and_then(|o| o.as_str().ok()) {
                    file_id = first;
                }
                
                println!("O len: {}, U len: {}, P: {}, ID len: {}", o_str.len(), u_str.len(), p_val, file_id.len());
                
                match SecurityHandler::new_v4("", o_str, u_str, p_val, file_id) {
                    Ok(handler) => {
                        println!("SecurityHandler V4 initialized successfully.");
                        security_handler = Some(handler);
                    }
                    Err(e) => println!("Failed to initialize SecurityHandler V4: {:?}", e),
                }
            } else if v_val == 5 && r_val == 5 {
                let mut file_id = &[][..];
                if let Ok(id_array) = doc.trailer.get(b"ID")
                    && let Ok(arr) = id_array.as_array()
                    && let Some(first) = arr.first().and_then(|o| o.as_str().ok()) {
                    file_id = first;
                }
                match SecurityHandler::new_v5("", "", file_id) {
                    Ok(handler) => {
                        println!("SecurityHandler V5 initialized successfully.");
                        security_handler = Some(handler);
                    }
                    Err(e) => println!("Failed to initialize SecurityHandler V5: {:?}", e),
                }
            } else {
                println!("Unsupported Encryption Version/Revision: V={}, R={}", v_val, r_val);
            }
        } else {
            println!("Encrypt entry found but not a dictionary.");
        }
    } else {
        println!("No Encrypt entry in trailer.");
    }

    if let Some(handler) = security_handler {
        let mut string_count = 0;
        let mut stream_count = 0;
        let mut fail_count = 0;

        let ids: Vec<lopdf::ObjectId> = doc.objects.keys().cloned().collect();

        for id in ids {
            if let Some(obj) = doc.objects.get_mut(&id) {
                decrypt_object_recursive(obj, id, &handler, &mut string_count, &mut stream_count, &mut fail_count);
            }
        }
        println!("Decryption stats: Strings={}, Streams={}, Fails={}", string_count, stream_count, fail_count);
        doc.trailer.remove(b"Encrypt");
        println!("Removed /Encrypt from trailer.");
    }
    Ok(())
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
            *string_count += 1;
            match handler.decrypt_bytes(s, id.0, id.1) {
                Ok(decrypted) => {
                    *s = decrypted;
                }
                Err(_) => *fail_count += 1,
            }
        }
        lopdf::Object::Stream(stream) => {
            *stream_count += 1;
            match handler.decrypt_bytes(&stream.content, id.0, id.1) {
                Ok(decrypted) => {
                    stream.content = decrypted;
                }
                Err(_) => *fail_count += 1,
            }
        }
        lopdf::Object::Array(arr) => {
            for item in arr {
                decrypt_object_recursive(item, id, handler, string_count, stream_count, fail_count);
            }
        }
        lopdf::Object::Dictionary(dict) => {
            // lopdf::Dictionary doesn't expose iter_mut easily, but it has it via Deref or methods
            for (_, item) in dict.iter_mut() {
                decrypt_object_recursive(item, id, handler, string_count, stream_count, fail_count);
            }
        }
        _ => {}
    }
}
