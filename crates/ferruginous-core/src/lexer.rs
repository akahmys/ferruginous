use crate::error::{PdfError, PdfResult};
use bytes::Bytes;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Delimiters
    LeftParenthesis,    // (
    RightParenthesis,   // )
    LeftAngleBracket,   // <
    RightAngleBracket,  // >
    LeftSquareBracket,  // [
    RightSquareBracket, // ]
    LeftCurlyBracket,   // {
    RightCurlyBracket,  // }
    Solidus,            // /
    Percent,            // %

    // Compound Delimiters
    LiteralString(Bytes), // (...) 
    HexString(Bytes),     // <...>
    DictionaryOpen,       // <<
    DictionaryClose,      // >>

    // Literals and Regular Text
    Name(Bytes),        // /Name
    Integer(i64),
    Real(f64),
    Keyword(Bytes),     // obj, endobj, stream, etc.
    
    // special
    Comment(Bytes),
}

pub struct Lexer {
    input: Bytes,
    pos: usize,
}

impl Lexer {
    pub fn new(input: Bytes) -> Self {
        Self { input, pos: 0 }
    }

    pub fn input_remaining(&self) -> &[u8] {
        &self.input[self.pos..]
    }

    pub fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    pub fn input_slice(&self, n: usize) -> Bytes {
        self.input.slice(self.pos..self.pos + n)
    }

    pub fn next_token(&mut self) -> PdfResult<Option<Token>> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Ok(None);
        }

        let c = self.input[self.pos];
        match c {
            b'%' => {
                self.lex_comment()?;
                self.next_token()
            }
            b'(' => self.lex_literal_string(),
            b')' => {
                self.pos += 1;
                Ok(Some(Token::RightParenthesis))
            }
            b'[' => {
                self.pos += 1;
                Ok(Some(Token::LeftSquareBracket))
            }
            b']' => {
                self.pos += 1;
                Ok(Some(Token::RightSquareBracket))
            }
            b'{' => {
                self.pos += 1;
                Ok(Some(Token::LeftCurlyBracket))
            }
            b'}' => {
                self.pos += 1;
                Ok(Some(Token::RightCurlyBracket))
            }
            b'<' => {
                if self.peek() == Some(b'<') {
                    self.pos += 2;
                    Ok(Some(Token::DictionaryOpen))
                } else {
                    self.lex_hex_string()
                }
            }
            b'>' => {
                if self.peek() == Some(b'>') {
                    self.pos += 2;
                    Ok(Some(Token::DictionaryClose))
                } else {
                    self.pos += 1;
                    Ok(Some(Token::RightAngleBracket))
                }
            }
            b'/' => self.lex_name(),
            _ if c.is_ascii_digit() || c == b'-' || c == b'.' => self.lex_number(),
            _ if is_regular_char(c) => self.lex_keyword_or_bool(),
            _ => Err(PdfError::Lexical { pos: self.pos, message: format!("Unexpected character: {}", c as char) }),
        }
    }

    pub fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && is_whitespace(self.input[self.pos]) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos + 1 < self.input.len() {
            Some(self.input[self.pos + 1])
        } else {
            None
        }
    }

    fn lex_literal_string(&mut self) -> PdfResult<Option<Token>> {
        let start = self.pos;
        self.pos += 1; // skip '('
        let mut depth = 1;
        let mut has_escapes = false;
        
        let content_start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            match c {
                b'(' => {
                    depth += 1;
                    self.pos += 1;
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        let content_end = self.pos;
                        self.pos += 1;
                        if !has_escapes {
                            return Ok(Some(Token::LiteralString(self.input.slice(content_start..content_end))));
                        } else {
                            // Re-parse with escapes to build a new buffer
                            self.pos = content_start;
                            return self.lex_literal_string_complex(start);
                        }
                    }
                    self.pos += 1;
                }
                b'\\' => {
                    has_escapes = true;
                    self.pos += 1;
                    if self.pos < self.input.len() {
                        self.pos += 1;
                    }
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        
        // EOF reached without matching ')' - return partial as per robustness principle
        let content_end = self.pos;
        if !has_escapes {
            Ok(Some(Token::LiteralString(self.input.slice(content_start..content_end))))
        } else {
            self.pos = content_start;
            self.lex_literal_string_complex(start)
        }
    }

    fn lex_literal_string_complex(&mut self, _start: usize) -> PdfResult<Option<Token>> {
        let mut depth = 1;
        let mut result = Vec::new();
        
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            match c {
                b'(' => {
                    depth += 1;
                    result.push(c);
                    self.pos += 1;
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += 1;
                        return Ok(Some(Token::LiteralString(Bytes::from(result))));
                    }
                    result.push(c);
                    self.pos += 1;
                }
                b'\\' => {
                    self.pos += 1;
                    if let Some(decoded) = self.decode_escape_sequence()? {
                        result.push(decoded);
                    }
                    if self.pos < self.input.len() {
                        self.pos += 1;
                    }
                }
                _ => {
                    result.push(c);
                    self.pos += 1;
                }
            }
        }
        Ok(Some(Token::LiteralString(Bytes::from(result))))
    }

    fn decode_escape_sequence(&mut self) -> PdfResult<Option<u8>> {
        if self.pos >= self.input.len() { return Ok(None); }
        let next = self.input[self.pos];
        match next {
            b'n' => Ok(Some(b'\n')),
            b'r' => Ok(Some(b'\r')),
            b't' => Ok(Some(b'\t')),
            b'b' => Ok(Some(b'\x08')),
            b'f' => Ok(Some(b'\x0c')),
            b'(' => Ok(Some(b'(')),
            b')' => Ok(Some(b')')),
            b'\\' => Ok(Some(b'\\')),
            b'\r' | b'\n' => {
                if next == b'\r' && self.peek() == Some(b'\n') { self.pos += 1; }
                Ok(None)
            }
            _ if next.is_ascii_digit() => Ok(Some(self.parse_octal_sequence(next))),
            _ => Ok(Some(next)),
        }
    }

    fn parse_octal_sequence(&mut self, initial: u8) -> u8 {
        let mut octal = initial - b'0';
        let mut count = 1;
        while count < 3 && self.pos + 1 < self.input.len() {
            let n = self.input[self.pos + 1];
            if (b'0'..=b'7').contains(&n) {
                octal = octal.wrapping_mul(8).wrapping_add(n - b'0');
                self.pos += 1;
                count += 1;
            } else {
                break;
            }
        }
        octal
    }

    fn lex_hex_string(&mut self) -> PdfResult<Option<Token>> {
        self.pos += 1; // skip '<'
        let mut result = Vec::new();
        let mut current_byte: Option<u8> = None;

        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c == b'>' {
                if let Some(high) = current_byte {
                    result.push(high << 4);
                }
                self.pos += 1;
                return Ok(Some(Token::HexString(Bytes::from(result))));
            }
            
            if is_whitespace(c) {
                self.pos += 1;
                continue;
            }

            let val = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => {
                    self.pos += 1;
                    continue;
                }
            };

            if let Some(high) = current_byte {
                result.push((high << 4) | val);
                current_byte = None;
            } else {
                current_byte = Some(val);
            }
            self.pos += 1;
        }

        // EOF reached without matching '>' - return what we have as per robustness principle
        if let Some(high) = current_byte {
            result.push(high << 4);
        }
        Ok(Some(Token::HexString(Bytes::from(result))))
    }

    fn lex_name(&mut self) -> PdfResult<Option<Token>> {
        let _start = self.pos;
        self.pos += 1; // skip '/'
        let mut has_escapes = false;
        
        let content_start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if is_whitespace(c) || is_delimiter(c) {
                break;
            }
            if c == b'#' {
                has_escapes = true;
                self.pos += 3;
            } else {
                self.pos += 1;
            }
        }

        let content_end = self.pos;
        if !has_escapes {
            Ok(Some(Token::Name(self.input.slice(content_start..content_end))))
        } else {
            // Re-parse with escapes
            self.pos = content_start;
            let mut result = Vec::new();
            while self.pos < content_end {
                let c = self.input[self.pos];
                if c == b'#' && self.pos + 2 < self.input.len() {
                    let h1 = self.input[self.pos + 1];
                    let h2 = self.input[self.pos + 2];
                    if let (Some(d1), Some(d2)) = (hex_to_val(h1), hex_to_val(h2)) {
                        result.push((d1 << 4) | d2);
                        self.pos += 3;
                        continue;
                    }
                }
                result.push(c);
                self.pos += 1;
            }
            Ok(Some(Token::Name(Bytes::from(result))))
        }
    }

    fn lex_comment(&mut self) -> PdfResult<Option<Token>> {
        let _start = self.pos;
        self.pos += 1; // skip '%'
        while self.pos < self.input.len() && self.input[self.pos] != b'\r' && self.input[self.pos] != b'\n' {
            self.pos += 1;
        }
        Ok(None)
    }

    fn lex_number(&mut self) -> PdfResult<Option<Token>> {
        let start = self.pos;
        let mut is_real = false;
        let mut seen_dot = false;

        // Handle leading sign
        if self.pos < self.input.len() && (self.input[self.pos] == b'-' || self.input[self.pos] == b'+') {
            self.pos += 1;
        }

        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c == b'.' {
                if seen_dot {
                    break;
                }
                seen_dot = true;
                is_real = true;
                self.pos += 1;
            } else if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        let s = std::str::from_utf8(&self.input[start..self.pos]).map_err(|_| PdfError::Lexical {
            pos: start,
            message: "Invalid UTF-8 in number".into(),
        })?;

        // Robustness: handle degenerate cases like ".", "-", "+", "-."
        if s == "." || s == "-" || s == "+" || s == "-." || s == "+." {
            return Ok(Some(Token::Real(0.0)));
        }

        if is_real {
            let val = s.parse::<f64>().map_err(|_| PdfError::Lexical {
                pos: start,
                message: "Invalid real number".into(),
            })?;
            Ok(Some(Token::Real(val)))
        } else {
            let val = s.parse::<i64>().map_err(|_| PdfError::Lexical {
                pos: start,
                message: "Invalid integer".into(),
            })?;
            Ok(Some(Token::Integer(val)))
        }
    }

    fn lex_keyword_or_bool(&mut self) -> PdfResult<Option<Token>> {
        let start = self.pos;
        while self.pos < self.input.len() && is_regular_char(self.input[self.pos]) {
            self.pos += 1;
        }
        Ok(Some(Token::Keyword(self.input.slice(start..self.pos))))
    }
}

fn is_whitespace(c: u8) -> bool {
    matches!(c, 0 | 9 | 10 | 12 | 13 | 32)
}

fn is_delimiter(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

fn is_regular_char(c: u8) -> bool {
    !is_whitespace(c) && !is_delimiter(c)
}

fn hex_to_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_basic() {
        let input = Bytes::from_static(b"%PDF-1.7\n1 0 obj\n<< /Type /Page >>\nendobj");
        let mut lexer = Lexer::new(input);
        
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::Integer(1));
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::Integer(0));
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::Keyword(Bytes::from_static(b"obj")));
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::DictionaryOpen);
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::Name(Bytes::from_static(b"Type")));
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::Name(Bytes::from_static(b"Page")));
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::DictionaryClose);
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::Keyword(Bytes::from_static(b"endobj")));
    }

    #[test]
    fn test_lex_strings() {
        let input = Bytes::from_static(b"(Literal string) <48656c6c6f> <41>");
        let mut lexer = Lexer::new(input);
        
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::LiteralString(Bytes::from_static(b"Literal string")));
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::HexString(Bytes::from_static(b"Hello")));
        assert_eq!(lexer.next_token().unwrap().unwrap(), Token::HexString(Bytes::from_static(b"A")));
    }
}
