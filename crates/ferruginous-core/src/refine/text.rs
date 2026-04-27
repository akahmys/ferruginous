use crate::font::FontResource;
use crate::lexer::Token;
use bytes::Bytes;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn restructure_content_stream(
    data: &[u8],
    fonts: &BTreeMap<String, Arc<FontResource>>,
) -> Bytes {
    let mut output = Vec::new();
    let mut stack = Vec::new();
    let mut current_font: Option<Arc<FontResource>> = None;

    let tokens = crate::lexer::tokenize(data);
    for token in tokens {
        match token {
            Token::Keyword(kw) => {
                handle_keyword(kw, &mut stack, &mut current_font, fonts, &mut output);
            }
            _ => stack.push(token),
        }
    }

    // Flush remaining
    for t in stack {
        write_token(&mut output, t);
    }

    Bytes::from(output)
}

fn handle_keyword(
    op: String,
    stack: &mut Vec<Token>,
    current_font: &mut Option<Arc<FontResource>>,
    fonts: &BTreeMap<String, Arc<FontResource>>,
    output: &mut Vec<u8>,
) {
    match op.as_str() {
        "Tf" => {
            let size_opt = match stack.pop() {
                Some(Token::Real(f)) => Some(f),
                Some(Token::Integer(i)) => Some(i as f64),
                Some(t) => {
                    stack.push(t);
                    None
                }
                None => None,
            };
            if let Some(size) = size_opt {
                if let Some(Token::Name(name_bytes)) = stack.pop() {
                    let name_str = String::from_utf8_lossy(&name_bytes).to_string();
                    if let Some(font) = fonts.get(&name_str) {
                        *current_font = Some(font.clone());
                    }
                    stack.push(Token::Name(name_bytes));
                    stack.push(Token::Real(size));
                } else {
                    stack.push(Token::Real(size));
                }
            }
        }
        "Tj" | "'" | "\"" => {
            if let Some(token) = stack.last_mut()
                && let Some(font) = current_font.as_ref()
            {
                let refined_bytes = match token {
                    Token::String(s) => Some(restructure_string(s, font)),
                    Token::Hex(s) => Some(restructure_string(s, font)),
                    _ => None,
                };
                if let Some(bytes) = refined_bytes {
                    *token = Token::Hex(Bytes::from(bytes));
                }
            }
        }
        "TJ" => {
            if let Some(pos) = stack.iter().rposition(|t| t == &Token::LeftArray)
                && let Some(font) = current_font.as_ref() {
                for token in &mut stack[pos + 1..] {
                    let refined_bytes = match token {
                        Token::String(s) => Some(restructure_string(s, font)),
                        Token::Hex(s) => Some(restructure_string(s, font)),
                        _ => None,
                    };
                    if let Some(bytes) = refined_bytes {
                        *token = Token::Hex(Bytes::from(bytes));
                    }
                }
            }
        }
        _ => {}
    }

    for t in stack.drain(..) {
        write_token(output, t);
    }
    output.extend_from_slice(op.as_bytes());
    output.push(b' ');
}

fn restructure_string(input: &[u8], font: &FontResource) -> Vec<u8> {
    if !font.has_any_mapping() {
        return input.to_vec();
    }

    let mut result = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let (consumed, unicode_opt) = font.decode_next(&input[i..]);
        if consumed == 0 {
            result.extend_from_slice(&input[i..]);
            break;
        }
        
        let original_bytes = &input[i..i + consumed];

        if let Some(u) = unicode_opt {
            let mut mapped = false;
            // For Type0/CIDFonts, we always want to map to Identity-H (2-byte CID)
            if let Some(c) = u.chars().next()
                && let Some(gid) = font.unicode_to_gid.get(&c) {
                result.push((gid >> 8) as u8);
                result.push((gid & 0xFF) as u8);
                mapped = true;
            }
            
            if !mapped
                && let Some(reverse) = &font.reverse_adj1_mapping
                && let Some(cid) = reverse.get(&u) {
                result.push((cid >> 8) as u8);
                result.push((cid & 0xFF) as u8);
                mapped = true;
            }

            if !mapped {
                result.extend_from_slice(original_bytes);
            }
        } else {
            result.extend_from_slice(original_bytes);
        }
        
        i += consumed;
    }
    result
}

fn write_token(output: &mut Vec<u8>, token: Token) {
    token.write_to(output);
}

pub fn recover_string(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16BE
        let utf16_data: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        String::from_utf16_lossy(&utf16_data)
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        // UTF-8 with BOM
        String::from_utf8_lossy(&bytes[3..]).to_string()
    } else {
        // Fallback to UTF-8 or PDFDocEncoding (simplified as UTF-8 lossy here)
        String::from_utf8_lossy(bytes).to_string()
    }
}
