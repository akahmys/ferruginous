//! Text Refinement: Heuristic character code recovery.

use ::chardetng::EncodingDetector;
use bytes::Bytes;

/// Heuristically recovers a UTF-8 string from raw PDF bytes.
///
/// This handles:
/// 1. UTF-16BE (Standard PDF Unicode strings)
/// 2. PDFDocEncoding / WinAnsi (Partial mappings)
/// 3. Ambiguous CJK encodings via `chardetng`
pub fn recover_string(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }

    // 1. Check for UTF-16BE BOM (0xFE, 0xFF)
    if input.len() >= 2 && input[0] == 0xFE && input[1] == 0xFF {
        return decode_utf16_be(&input[2..]);
    }

    // 2. Check for UTF-8 (Common in newer or non-compliant PDFs)
    if let Ok(s) = std::str::from_utf8(input) {
        return s.to_string();
    }

    // 3. Fallback to heuristic detection using chardetng
    let mut detector = EncodingDetector::new();
    detector.feed(input, true);
    let (encoding, _) = (detector.guess(None, true), true); // Simplify for now

    let (decoded, ..) = encoding.decode(input);
    decoded.into_owned()
}

fn decode_utf16_be(bytes: &[u8]) -> String {
    let u16s: Vec<u16> =
        bytes.chunks_exact(2).map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]])).collect();
    String::from_utf16_lossy(&u16s)
}

/// Helper for sanitizing strings back to Bytes (UTF-8).
pub fn refine_string(input: &[u8]) -> Bytes {
    let recovered = recover_string(input);
    Bytes::from(recovered.into_bytes())
}

use crate::font::FontResource;
use crate::lexer::{Lexer, Token};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Restructures a content stream by rewriting all text show operators to use UTF-8.
/// This requires a map of font resources defined in the stream's context.
pub fn restructure_content_stream(
    data: &[u8],
    fonts: &BTreeMap<String, Arc<FontResource>>,
) -> Bytes {
    let mut lexer = Lexer::new(Bytes::copy_from_slice(data));
    let mut output = Vec::new();
    let mut current_font: Option<Arc<FontResource>> = None;
    let mut stack = Vec::new();

    while let Ok(token) = lexer.next() {
        if token == Token::EOF {
            break;
        }

        if let Token::Keyword(op) = token {
            handle_operator(op, &mut stack, &mut current_font, fonts, &mut output);
        } else {
            stack.push(token);
        }
    }

    for t in stack.drain(..) {
        write_token(&mut output, t);
    }

    Bytes::from(output)
}

fn handle_operator(
    op: String,
    stack: &mut Vec<Token>,
    current_font: &mut Option<Arc<FontResource>>,
    fonts: &BTreeMap<String, Arc<FontResource>>,
    output: &mut Vec<u8>,
) {
    match op.as_str() {
        "Tf" => {
            if stack.len() >= 2
                && let Some(Token::Name(font_name)) = stack.get(stack.len() - 2)
            {
                *current_font = fonts.get(font_name).cloned();
            }
        }
        "Tj" | "'" | "\"" => {
            if let Some(Token::String(s)) = stack.last_mut()
                && let Some(font) = current_font.as_ref()
            {
                let utf8_string = restructure_string(s, font);
                *s = Bytes::from(utf8_string);
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

fn restructure_string(input: &[u8], font: &FontResource) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < input.len() {
        let (consumed, _) = font.decode_next(&input[i..]);
        if consumed == 0 {
            break;
        }
        let code = &input[i..i + consumed];

        // Use the unified map (Unicode based)
        // If we don't have a direct mapping in the unified map, we use our best guess
        // and potentially assign a PUA code if we want to be strict.
        // Use the unified map (Unicode based)
        // If we don't have a direct mapping in the unified map, we use our best guess
        // and potentially assign a PUA code if we want to be strict.
        let _bytes_key = if consumed == 1 {
            code[0] as u32
        } else if consumed == 2 {
            ((code[0] as u32) << 8) | (code[1] as u32)
        } else {
            0
        };

        // Find the Unicode in the unified_map based on CID
        let cid = font.to_cid(code);
        let found = font.unified_map.iter().find(|&(_, &v)| v == cid).map(|(k, _)| k.as_str());

        if let Some(u) = found {
            result.push_str(u);
        } else {
            result.push('\u{FFFD}');
        }

        i += consumed;
    }
    result
}

fn write_token(output: &mut Vec<u8>, token: Token) {
    match token {
        Token::Boolean(b) => output.extend_from_slice(if b { b"true " } else { b"false " }),
        Token::Integer(i) => {
            output.extend_from_slice(i.to_string().as_bytes());
            output.push(b' ');
        }
        Token::Real(f) => {
            output.extend_from_slice(f.to_string().as_bytes());
            output.push(b' ');
        }
        Token::String(s) => {
            output.push(b'(');
            // Simple escaping
            for &b in &s {
                match b {
                    b'(' | b')' | b'\\' => {
                        output.push(b'\\');
                        output.push(b);
                    }
                    _ => output.push(b),
                }
            }
            output.extend_from_slice(b") ");
        }
        Token::Name(n) => {
            output.push(b'/');
            output.extend_from_slice(n.as_bytes());
            output.push(b' ');
        }
        Token::Keyword(k) => {
            output.extend_from_slice(k.as_bytes());
            output.push(b' ');
        }
        Token::LeftArray => output.push(b'['),
        Token::RightArray => output.extend_from_slice(b"] "),
        Token::LeftDict => output.extend_from_slice(b"<< "),
        Token::RightDict => output.extend_from_slice(b">> "),
        Token::Comment(c) => {
            output.push(b'%');
            output.extend_from_slice(c.as_bytes());
            output.push(b'\n');
        }
        Token::Null => output.extend_from_slice(b"null "),
        Token::EOF => {}
    }
}
