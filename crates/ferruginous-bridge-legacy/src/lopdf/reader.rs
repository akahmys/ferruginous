use crate::lopdf::Document;
use crate::lopdf::xref::{Xref, XrefEntry};

pub struct Reader;

impl Reader {
    pub fn find_header(data: &[u8]) -> Option<usize> {
        let tag = b"%PDF-";
        data.windows(tag.len()).position(|window| window == tag)
    }

    pub fn find_eof(data: &[u8]) -> Option<usize> {
        let tag = b"%%EOF";
        // Search from the end for the last %%EOF
        for i in (0..data.len().saturating_sub(tag.len())).rev() {
            if &data[i..i + tag.len()] == tag {
                return Some(i);
            }
        }
        None
    }

    /// Reconstructs the xref table by scanning the entire file for "obj" tags.
    /// This is slow but necessary for severely damaged legacy files.
    pub fn reconstruct_xref(data: &[u8]) -> Xref {
        let mut xref = Xref::new();
        let obj_tag = b" obj";
        
        let mut i = 0;
        while i < data.len().saturating_sub(obj_tag.len()) {
            if &data[i..i + obj_tag.len()] == obj_tag {
                // Found something that looks like an object definition end.
                // We need to look back for the ID and Generation.
                // For now, this is a simplified version.
                let mut start = i;
                while start > 0 && (data[start-1].is_ascii_digit() || data[start-1].is_ascii_whitespace()) {
                    start -= 1;
                }
                
                let snippet = &data[start..i];
                let parts: Vec<&[u8]> = snippet.split(|&b| b.is_ascii_whitespace())
                    .filter(|p| !p.is_empty())
                    .collect();
                
                if parts.len() >= 2 {
                    if let (Ok(id), Ok(gen)) = (
                        std::str::from_utf8(parts[0]).unwrap_or("").parse::<u32>(),
                        std::str::from_utf8(parts[1]).unwrap_or("").parse::<u16>()
                    ) {
                        xref.insert(id, XrefEntry::Normal { offset: start as u64, generation: gen });
                    }
                }
            }
            i += 1;
        }
        xref
    }

    pub fn load_document(data: &[u8]) -> Result<Document, crate::BridgeError> {
        let mut doc = Document::new();
        
        let header_offset = Self::find_header(data).ok_or_else(|| crate::BridgeError::Parse("No PDF header found".into()))?;
        if header_offset > 0 {
            doc.repair_log.push(format!("Found PDF header at offset {}. Repairing start position.", header_offset));
        }
        let header_str = std::str::from_utf8(&data[header_offset..header_offset+8]).unwrap_or("%PDF-1.7");
        doc.version = header_str[5..].to_string();

        // Check for Linearization (Fast Web View)
        let search_limit = std::cmp::min(data.len(), header_offset + 1024);
        let lin_tag = b"/Linearized";
        if data[header_offset..search_limit].windows(lin_tag.len()).position(|w| w == lin_tag).is_some() {
            doc.repair_log.push("Linearized PDF detected. Handling optimized structure via global scan.".to_string());
        }

        // For now, let's just reconstruct the xref to be safe (the "dirty" way)
        doc.repair_log.push("Reconstructing xref table via full file scan (Dirty repair mode).".to_string());
        doc.xref = Self::reconstruct_xref(data);
        doc.repair_log.push(format!("Found {} objects during physical scan.", doc.xref.entries.len()));

        // Find trailer
        let eof_pos = Self::find_eof(data).unwrap_or(data.len());
        let trailer_tag = b"trailer";
        let mut trailer_pos = None;
        for i in (0..eof_pos.saturating_sub(trailer_tag.len())).rev() {
            if &data[i..i+trailer_tag.len()] == trailer_tag {
                trailer_pos = Some(i + trailer_tag.len());
                break;
            }
        }

        if let Some(pos) = trailer_pos {
            match super::parser::Parser::parse_dictionary(&data[pos..]) {
                Ok((_rest, dict)) => {
                    doc.repair_log.push(format!("Successfully recovered trailer dictionary with {} keys.", dict.len()));
                    doc.trailer = dict;
                }
                Err(e) => {
                    doc.repair_log.push(format!("Failed to parse recovered trailer: {:?}. Attempting root resolution manually...", e));
                }
            }
        } else {
            doc.repair_log.push("CRITICAL: 'trailer' keyword not found. Document catalog might be missing.".to_string());
        }

        // Initialize decryptor if /Encrypt exists
        let mut decryptor = None;
        if let Some(encrypt_obj) = doc.trailer.get(b"Encrypt".as_slice()) {
            let ref_id_copy = if let super::Object::Reference(r) = doc.resolve(encrypt_obj) {
                Some(*r)
            } else {
                None
            };

            if let Some(ref_id) = ref_id_copy {
                doc.repair_log.push(format!("Legacy encryption detected (Object {}). Initializing RC4 decryptor.", ref_id.id));
                if let Some(encrypt_dict_obj) = doc.get_object(ref_id.id) {
                    if let Some(dict) = encrypt_dict_obj.as_dict() {
                        let o = dict.get(b"O".as_slice()).and_then(|obj| obj.as_str()).unwrap_or(&[]);
                        let p = dict.get(b"P".as_slice()).and_then(|obj| obj.as_i64()).unwrap_or(0) as i32;
                        let id_arr: &[super::Object] = doc.trailer.get(b"ID".as_slice()).and_then(|obj| obj.as_array()).unwrap_or(&[]);
                        let id = if !id_arr.is_empty() { id_arr[0].as_str().unwrap_or(&[]) } else { &[] };
                        
                        let password = b""; 
                        let key = crate::lopdf::encryption::algorithms::derive_key_v2(password, o, p, id, true);
                        decryptor = Some(crate::lopdf::encryption::Decryptor::new(crate::lopdf::encryption::Algorithm::Rc4, &key));
                    }
                }
            }
        }

        // Parse all objects found in the xref
        for (id, entry) in &doc.xref.entries {
            if let XrefEntry::Normal { offset, .. } = entry {
                let start = *offset as usize;
                if let Ok((rest, (obj_id, gen))) = super::parser::Parser::parse_object_id(&data[start..]) {
                    if obj_id == *id {
                        if let Ok((_, mut obj)) = super::parser::Parser::parse_object(rest) {
                            // Apply decryption if necessary
                            if let Some(ref d) = decryptor {
                                Self::decrypt_object(&mut obj, d, obj_id, gen);
                            }
                            doc.objects.insert(obj_id, obj);
                        }
                    }
                }
            }
        }

        Ok(doc)
    }

    fn decrypt_object(obj: &mut super::Object, d: &crate::lopdf::encryption::Decryptor, id: u32, gen: u16) {
        match obj {
            super::Object::String(bytes, _format) => {
                let decrypted = d.decrypt(id, gen, bytes);
                *bytes = bytes::Bytes::from(decrypted);
            }
            super::Object::Stream(stream) => {
                let decrypted = d.decrypt(id, gen, &stream.content);
                stream.content = bytes::Bytes::from(decrypted);
                // Also decrypt strings in stream dictionary
                for (_, val) in stream.dict.iter_mut() {
                    Self::decrypt_object(val, d, id, gen);
                }
            }
            super::Object::Array(arr) => {
                for item in arr.iter_mut() {
                    Self::decrypt_object(item, d, id, gen);
                }
            }
            super::Object::Dictionary(dict) => {
                for (_, val) in dict.iter_mut() {
                    Self::decrypt_object(val, d, id, gen);
                }
            }
            _ => {}
        }
    }
}
