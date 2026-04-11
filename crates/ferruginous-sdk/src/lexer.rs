//! Lexical analysis for PDF content streams and objects.
//! (ISO 32000-2:2020 Clause 7.2)

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1, take_while_m_n},
    character::complete::{char as cchar, digit1},
    combinator::{map, map_res, opt, recognize, value},
    multi::{many0, many_till},
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use nom;
use std::collections::BTreeMap;
use crate::core::{Object, Reference};

/// ISO 32000-2:2020 Clause 7.2.2 - Whitespace characters
pub(crate) const fn is_pdf_whitespace(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}

/// ISO 32000-2:2020 Clause 7.2.3 - Comments
fn skip_comment(input: &[u8]) -> IResult<&[u8], ()> {
    debug_assert!(!input.is_empty(), "skip_comment: input empty");
    debug_assert!(input[0] == b'%', "skip_comment: must start with %");
    let (input, _) = cchar('%')(input)?;
    let (input, _) = take_while(|b| b != b'\n' && b != b'\r')(input)?;
    Ok((input, ()))
}

/// A parser that consumes whitespace and comments
pub(crate) fn pdf_multispace0(input: &[u8]) -> IResult<&[u8], ()> {
    let mut current_input = input;
    let mut loop_count = 0;
    const MAX_WS_ITER: usize = 100_000;
    loop {
        loop_count += 1;
        debug_assert!(loop_count < MAX_WS_ITER, "pdf_multispace0: loop limit reached");
        if loop_count > MAX_WS_ITER { return Ok((current_input, ())); }

        let (next_input, _) = take_while(is_pdf_whitespace)(current_input)?;
        if next_input.first() == Some(&b'%') {
            let (next_input, ()) = skip_comment(next_input)?;
            current_input = next_input;
        } else {
            return Ok((next_input, ()));
        }
    }
}

/// ISO 32000-2:2020 Clause 7.2.2 - Delimiter characters
pub(crate) const fn is_pdf_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

/// ISO 32000-2:2020 Clause 7.3.2 - Boolean objects
fn parse_boolean(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_boolean: input empty");
    alt((
        value(Object::Boolean(true), tag("true")),
        value(Object::Boolean(false), tag("false")),
    ))(input)
}

/// ISO 32000-2:2020 Clause 7.3.3 - Numeric objects (Integer)
fn parse_integer(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_integer: input empty");
    debug_assert!(input[0].is_ascii_digit() || input[0] == b'+' || input[0] == b'-', "parse_integer: invalid start (pre-verify)");
    let (input, (sign, digits)) = pair(opt(alt((tag("+"), tag("-")))), digit1)(input)?;
    let mut s = String::new();
    if let Some(sig) = sign {
        s.push(if sig == b"+" { '+' } else { '-' });
    }
    s.push_str(std::str::from_utf8(digits).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?);
    let n = s.parse::<i64>().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    Ok((input, Object::Integer(n)))
}

/// ISO 32000-2:2020 Clause 7.3.3 - Numeric objects (Real)
fn parse_real(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_real: input empty");
    debug_assert!(input[0].is_ascii_digit() || input[0] == b'+' || input[0] == b'-' || input[0] == b'.', "parse_real: invalid start (pre-verify)");
    let (input, res) = recognize(tuple((
        opt(alt((tag("+"), tag("-")))),
        alt((
            recognize(pair(digit1, pair(tag("."), opt(digit1)))),
            recognize(pair(tag("."), digit1)),
        )),
    )))(input)?;
    let s = std::str::from_utf8(res).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float)))?;
    let n = s.parse::<f64>().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float)))?;
    Ok((input, Object::Real(n)))
}

/// Helper for parsing escape sequences in literal strings (Clause 7.3.4.2)
fn parse_escape(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    debug_assert!(!input.is_empty(), "parse_escape: input empty");
    debug_assert!(input[0] == b'\\', "parse_escape: must start with backslash");
    let (input, _) = tag("\\")(input)?;
    alt((
        value(vec![b'\n'], tag("n")),
        value(vec![b'\r'], tag("r")),
        value(vec![b'\t'], tag("t")),
        value(vec![0x08], tag("b")),
        value(vec![0x0C], tag("f")),
        value(vec![b'('], tag("(")),
        value(vec![b')'], tag(")")),
        value(vec![b'\\'], tag("\\")),
        // Octal escape: \ddd (1 to 3 digits)
        map_res(
            take_while_m_n(1, 3, |b: u8| (b'0'..=b'7').contains(&b)),
            |s: &[u8]| {
                let s_str = std::str::from_utf8(s).map_err(|_| ())?;
                u8::from_str_radix(s_str, 8).map(|b| vec![b]).map_err(|_| ())
            }
        ),
        // Line continuation: \ followed by newline
        value(vec![], alt((tag("\r\n"), tag("\n"), tag("\r")))),
        // Invalid escape: just ignore the backslash
        map(nom::character::complete::anychar, |c| vec![c as u8]),
    ))(input)
}

/// ISO 32000-2:2020 Clause 7.3.4.2 - Literal Strings (Non-recursive)
fn parse_literal_string(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_literal_string: input empty");
    let (input, _) = tag("(")(input)?;
    debug_assert!(input.len() < 10 * 1024 * 1024, "parse_literal_string: excessive string length context");
    let mut content = Vec::new();
    let mut depth = 1;
    let mut current_input = input;

    let mut loop_count = 0;
    const MAX_STR_LEN: usize = 1_000_000;

    while depth > 0 {
        loop_count += 1;
        debug_assert!(loop_count < MAX_STR_LEN, "parse_literal_string: loop limit reached");
        if loop_count > MAX_STR_LEN { return Err(nom::Err::Error(nom::error::Error::new(current_input, nom::error::ErrorKind::TooLarge))); }
        if current_input.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(current_input, nom::error::ErrorKind::Eof)));
        }

        if current_input.starts_with(b"\\") {
            let (next_input, mut bytes) = parse_escape(current_input)?;
            content.append(&mut bytes);
            current_input = next_input;
        } else if current_input.starts_with(b"(") {
            content.push(b'(');
            depth += 1;
            current_input = &current_input[1..];
        } else if current_input.starts_with(b")") {
            depth -= 1;
            if depth > 0 {
                content.push(b')');
            }
            current_input = &current_input[1..];
        } else {
            let (next_input, chunk) = take_while1(|b| b != b'(' && b != b')' && b != b'\\')(current_input)?;
            content.extend_from_slice(chunk);
            current_input = next_input;
        }
    }

    Ok((current_input, Object::new_string(content)))
}

/// ISO 32000-2:2020 Clause 7.3.4.3 - Hexadecimal Strings
fn parse_hex_string(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_hex_string: input empty");
    let (input, content) = delimited(
        tag("<"),
        take_while(|b: u8| b.is_ascii_hexdigit() || is_pdf_whitespace(b)),
        tag(">")
    )(input)?;
    
    // Security: Limit hex string content to 10MB
    const MAX_HEX_STR_LEN: usize = 10 * 1024 * 1024;
    if content.len() > MAX_HEX_STR_LEN {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::TooLarge)));
    }
    
    let cleaned: Vec<u8> = content.iter().filter(|b| !is_pdf_whitespace(**b)).copied().collect();
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    let mut i = 0;
    while i < cleaned.len() {
        let mut tmp = [0u8; 2];
        let hex = if i + 2 <= cleaned.len() {
            &cleaned[i..i+2]
        } else {
            // Odd number of digits: append 0 (Clause 7.3.4.3)
            tmp[0] = cleaned[i];
            tmp[1] = b'0';
            &tmp
        };
        
        let s = std::str::from_utf8(hex).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::HexDigit)))?;
        let b = u8::from_str_radix(s, 16).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::HexDigit)))?;
        bytes.push(b);
        i += 2;
    }
    Ok((input, Object::new_string(bytes)))
}

/// ISO 32000-2:2020 Clause 7.3.5 - Name objects
pub fn parse_name(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_name: input empty");
    let (input, _) = tag("/")(input)?;
    let (input, raw_name) = take_while(|b: u8| !is_pdf_whitespace(b) && !is_pdf_delimiter(b))(input)?;
    debug_assert!(raw_name.len() < 256, "parse_name: name too long per PDF spec limit recommended");
    
    let mut decoded = Vec::new();
    let mut i = 0;
    while i < raw_name.len() {
        let b = raw_name[i];
        if b == b'#' && i + 2 < raw_name.len() {
            let hex = &raw_name[i+1..i+3];
            let hex_str = std::str::from_utf8(hex).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::HexDigit)))?;
            if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                decoded.push(val);
                i += 3;
            } else {
                decoded.push(b);
                i += 1;
            }
        } else {
            decoded.push(b);
            i += 1;
        }
    }
    Ok((input, Object::new_name(decoded)))
}

/// ISO 32000-2:2020 Clause 7.3.6 - Array objects
fn parse_array(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_array: input empty");
    let (input, _) = tag("[")(input)?;
    debug_assert!(input.len() < 100 * 1024 * 1024, "parse_array: context too large");
    let (input, elements) = many0(parse_object)(input)?;
    let (input, _) = preceded(pdf_multispace0, tag("]"))(input)?;
    Ok((input, Object::new_array(elements)))
}

/// ISO 32000-2:2020 Clause 7.3.7 - Dictionary objects
pub fn parse_dictionary(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_dictionary: input empty");
    let (input, _) = tag("<<")(input)?;
    debug_assert!(input.len() >= 2, "parse_dictionary: input truncated after open-tag");
    let (input, pairs) = many0(pair(preceded(pdf_multispace0, parse_name), parse_object))(input)?;
    let (input, _) = preceded(pdf_multispace0, tag(">>"))(input)?;
    
    let mut dict = BTreeMap::new();
    for (name_obj, val) in pairs {
        if let Some(name) = name_obj.as_str() {
            dict.insert(name.to_vec(), val);
        }
    }
    Ok((input, Object::new_dict(dict)))
}

/// ISO 32000-2:2020 Clause 7.3.9 - Null object
fn parse_null(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_null: input empty");
    value(Object::Null, tag("null"))(input)
}

/// ISO 32000-2:2020 Clause 7.3.10 - Indirect Objects (References)
fn parse_reference(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_reference: input empty");
    let (input, id_bytes) = digit1(input)?;
    let id: u32 = std::str::from_utf8(id_bytes).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?.parse().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let (input, _) = take_while1(is_pdf_whitespace)(input)?;
    let (input, generation_bytes) = digit1(input)?;
    let generation: u16 = std::str::from_utf8(generation_bytes).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?.parse().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let (input, _) = take_while1(is_pdf_whitespace)(input)?;
    let (input, _) = tag("R")(input)?;
    Ok((input, Object::Reference(Reference { id, generation })))
}

/// ISO 32000-2:2020 Clause 7.3.8 - Stream objects
fn parse_stream(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_stream: input empty");
    let (input, dict_obj) = parse_dictionary(input)?;
    debug_assert!(matches!(dict_obj, Object::Dictionary(_)), "parse_stream: expected dictionary before stream tag");
    let (input, ()) = pdf_multispace0(input)?;
    let (input, _) = tag("stream")(input)?;
    // Liberal Read: ISO says \r\n or \n, but we allow \r and optional spaces for robustness.
    let (input, _) = take_while(|b| matches!(b, b' ' | b'\t'))(input)?;
    let (input, _) = alt((tag("\r\n"), tag("\n"), tag("\r")))(input)?;
    
    let Object::Dictionary(ref dict) = dict_obj else {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)));
    };
    
    if let Some(Object::Integer(len)) = dict.get(b"Length".as_slice()) {
        let len_usize = *len as usize;
        if input.len() >= len_usize {
            let bytes = input[..len_usize].to_vec();
            let remaining = &input[len_usize..];
            let (remaining, ()) = pdf_multispace0(remaining)?;
            let (remaining, _) = tag("endstream")(remaining)?;
            return Ok((remaining, Object::new_stream_arc(std::sync::Arc::clone(dict), std::sync::Arc::new(bytes))));
        }
    }

    // Fallback for cases where Length is not yet resolved or missing (though required by spec)
    // Security: Limit search to 10MB to avoid excessive scanning
    const MAX_STREAM_SEARCH: usize = 10 * 1024 * 1024;
    let search_data = if input.len() > MAX_STREAM_SEARCH { &input[..MAX_STREAM_SEARCH] } else { input };

    let (input, (content, _)) = many_till(nom::character::complete::anychar, preceded(opt(take_while(|b| matches!(b, b' ' | b'\t'))), preceded(alt((tag("\r\n"), tag("\n"), tag("\r"))), tag("endstream"))))(search_data)?;
    let bytes: Vec<u8> = content.into_iter().map(|c| c as u8).collect();
    
    Ok((input, Object::new_stream_arc(std::sync::Arc::clone(dict), std::sync::Arc::new(bytes))))
}

/// Entry point for parsing any object
pub fn parse_object(input: &[u8]) -> IResult<&[u8], Object> {
    debug_assert!(!input.is_empty(), "parse_object: input should not be empty");
    debug_assert!(input.len() < 1024 * 1024 * 1024, "parse_object: input too large");
    let (input, ()) = pdf_multispace0(input)?;
    if input.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Eof)));
    }
    
    match input[0] {
        b't' | b'f' => parse_boolean(input),
        b'n' => parse_null(input),
        b'0'..=b'9' | b'+' | b'-' | b'.' => {
            // Numbers or References
            alt((parse_reference, parse_real, parse_integer))(input)
        }
        b'/' => parse_name(input),
        b'<' => {
            if input.get(1) == Some(&b'<') {
                alt((parse_stream, parse_dictionary))(input)
            } else {
                parse_hex_string(input)
            }
        }
        b'(' => parse_literal_string(input),
        b'[' => parse_array(input),
        _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))),
    }
}

/// Parses a content stream operator (e.g., "q", "m", "BT")
/// (Clause 7.2.1 - Lexical Rules)
pub fn parse_operator(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    debug_assert!(input.len() < 1024 * 1024, "parse_operator: input chunk too large");
    let (input, ()) = pdf_multispace0(input)?;
    let (input, op) = take_while1(|b: u8| !is_pdf_whitespace(b) && !is_pdf_delimiter(b))(input)?;
    debug_assert!(!op.is_empty(), "parse_operator: operator name is empty");
    // Ensure it's not a keyword like true/false/null which are Objects
    if op == b"true" || op == b"false" || op == b"null" {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)));
    }
    Ok((input, op.to_vec()))
}

/// Parses an indirect object header: `<id> <gen> obj`
/// (Clause 7.3.10 - Indirect Objects)
pub fn parse_id_gen_obj(input: &[u8]) -> IResult<&[u8], (u32, u16)> {
    debug_assert!(!input.is_empty(), "parse_id_gen_obj: input empty");
    let (input, ()) = pdf_multispace0(input)?;
    debug_assert!(input.len() > 3, "parse_id_gen_obj: input too short for header");
    let (input, id_bytes) = digit1(input)?;
    let id: u32 = std::str::from_utf8(id_bytes).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?.parse().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let (input, _) = take_while1(is_pdf_whitespace)(input)?;
    let (input, generation_bytes) = digit1(input)?;
    let generation: u16 = std::str::from_utf8(generation_bytes).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?.parse().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let (input, _) = take_while1(is_pdf_whitespace)(input)?;
    let (input, _) = tag("obj")(input)?;
    let (input, ()) = pdf_multispace0(input)?;
    Ok((input, (id, generation)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_complex_objects() -> Result<(), Box<dyn std::error::Error>> {
        // Array
        assert_eq!(parse_object(b"[1 2 /Name]").map_err(|e| e.to_string())?.1, 
            Object::new_array(vec![Object::Integer(1), Object::Integer(2), Object::new_name(b"Name".to_vec())]));
 
        // Dictionary
        let dict_input = b"<< /Type /Catalog /Pages 2 0 R >>";
        let (_, obj) = parse_object(dict_input).map_err(|e| e.to_string())?;
        if let Object::Dictionary(d) = obj {
            assert_eq!(d.get(b"Type".as_slice()).ok_or("Missing Type")?, &Object::new_name(b"Catalog".to_vec()));
            assert_eq!(d.get(b"Pages".as_slice()).ok_or("Missing Pages")?, &Object::Reference(Reference { id: 2, generation: 0 }));
        } else {
            panic!("Expected Dictionary");
        }

        // Hex String
        assert_eq!(parse_object(b"<4E6F76>").map_err(|e| e.to_string())?.1, Object::new_string(b"Nov".to_vec()));

        // Stream
        let stream_input = b"<< /Length 5 >>\nstream\nabcde\nendstream";
        let (_, obj) = parse_object(stream_input).map_err(|e| e.to_string())?;
        if let Object::Stream(d, b) = obj {
            assert_eq!(d.get(b"Length".as_slice()).ok_or("Missing Length")?, &Object::Integer(5));
            assert_eq!(b.as_ref(), b"abcde");
        } else {
            panic!("Expected Stream");
        }
        Ok(())
    }

    #[test]
    fn test_literal_string_advanced() -> Result<(), Box<dyn std::error::Error>> {
        // Nested parens
        assert_eq!(parse_object(b"(one (two) three)").map_err(|e| e.to_string())?.1, Object::new_string(b"one (two) three".to_vec()));
        // Escapes
        assert_eq!(parse_object(b"(\\n\\r\\t\\b\\f)").map_err(|e| e.to_string())?.1, Object::new_string(b"\n\r\t\x08\x0C".to_vec()));
        assert_eq!(parse_object(b"(\\( \\) \\\\)").map_err(|e| e.to_string())?.1, Object::new_string(b"( ) \\".to_vec()));
        // Octal
        assert_eq!(parse_object(b"(\\101\\102\\103)").map_err(|e| e.to_string())?.1, Object::new_string(b"ABC".to_vec()));
        // Line continuation
        assert_eq!(parse_object(b"(abc\\\ndef)").map_err(|e| e.to_string())?.1, Object::new_string(b"abcdef".to_vec()));
        Ok(())
    }
}
