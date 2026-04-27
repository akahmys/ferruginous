//! ISO 32000-2:2020 Clause 7.2 - Lexical Conventions

use crate::PdfResult;
use bytes::Bytes;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(Bytes),
    Hex(Bytes),
    Name(Bytes),
    Keyword(String),
    LeftArray,
    RightArray,
    LeftDict,
    RightDict,
    Comment(String),
    Null,
    EOF,
}

impl Token {
    pub fn write_to(&self, output: &mut Vec<u8>) {
        match self {
            Token::Boolean(b) => output.extend_from_slice(if *b { b"true " } else { b"false " }),
            Token::Integer(i) => output.extend_from_slice(format!("{} ", i).as_bytes()),
            Token::Real(f) => output.extend_from_slice(format!("{:.4} ", f).as_bytes()),
            Token::String(s) => {
                output.push(b'(');
                for &b in s {
                    if b == b'(' || b == b')' || b == b'\\' {
                        output.push(b'\\');
                    }
                    output.push(b);
                }
                output.push(b')');
                output.push(b' ');
            }
            Token::Hex(s) => {
                output.push(b'<');
                for &b in s {
                    output.extend_from_slice(format!("{:02X}", b).as_bytes());
                }
                output.push(b'>');
                output.push(b' ');
            }
            Token::Name(n) => {
                output.push(b'/');
                for &b in n {
                    if b == b'#' || b <= 32 || b >= 127 || is_delimiter(b) {
                        output.extend_from_slice(format!("#{b:02X}").as_bytes());
                    } else {
                        output.push(b);
                    }
                }
                output.push(b' ');
            }
            Token::Keyword(kw) => {
                output.extend_from_slice(kw.as_bytes());
                output.push(b' ');
            }
            Token::LeftArray => output.extend_from_slice(b"[ "),
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
}

/// Convenience function to tokenize a buffer.
pub fn tokenize(data: &[u8]) -> Vec<Token> {
    let mut lexer = Lexer::new(Bytes::copy_from_slice(data));
    let mut tokens = Vec::new();
    while let Ok(token) = lexer.next_token() {
        if token == Token::EOF {
            break;
        }
        tokens.push(token);
    }
    tokens
}

pub struct Lexer {
    data: Bytes,
    pos: usize,
}

impl Lexer {
    pub fn new(data: Bytes) -> Self {
        Self { data, pos: 0 }
    }

    pub fn next_token(&mut self) -> PdfResult<Token> {
        self.skip_whitespace_and_comments();
        if self.pos >= self.data.len() {
            return Ok(Token::EOF);
        }

        let b = self.data[self.pos];
        match b {
            b'/' => self.lex_name(),
            b'(' => self.lex_literal_string(),
            b'<' => {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'<' {
                    self.pos += 2;
                    Ok(Token::LeftDict)
                } else {
                    self.lex_hex_string()
                }
            }
            b'>' => {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'>' {
                    self.pos += 2;
                    Ok(Token::RightDict)
                } else {
                    self.pos += 1;
                    Ok(Token::Keyword(">".to_string()))
                }
            }
            b'[' => {
                self.pos += 1;
                Ok(Token::LeftArray)
            }
            b']' => {
                self.pos += 1;
                Ok(Token::RightArray)
            }
            b'{' => {
                self.pos += 1;
                Ok(Token::Keyword("{".to_string()))
            }
            b'}' => {
                self.pos += 1;
                Ok(Token::Keyword("}".to_string()))
            }
            b'0'..=b'9' | b'+' | b'-' | b'.' => self.lex_number_or_keyword(),
            _ => self.lex_keyword_or_other(),
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if is_whitespace(b) {
                self.pos += 1;
            } else if b == b'%' {
                self.pos += 1;
                while self.pos < self.data.len() && !is_newline(self.data[self.pos]) {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn lex_name(&mut self) -> PdfResult<Token> {
        self.pos += 1; // skip '/'
        let mut result = Vec::new();
        while self.pos < self.data.len()
            && !is_delimiter(self.data[self.pos])
            && !is_whitespace(self.data[self.pos])
        {
            let b = self.data[self.pos];
            if b == b'#' && self.pos + 2 < self.data.len() {
                let hex = &self.data[self.pos + 1..self.pos + 3];
                if let Ok(utf8_str) = std::str::from_utf8(hex)
                    && let Ok(val) = u8::from_str_radix(utf8_str, 16) {
                    result.push(val);
                    self.pos += 3;
                    continue;
                }
            }
            result.push(b);
            self.pos += 1;
        }
        Ok(Token::Name(Bytes::from(result)))
    }

    fn lex_literal_string(&mut self) -> PdfResult<Token> {
        self.pos += 1; // skip '('
        let mut balance = 1;
        let mut result = Vec::new();
        while self.pos < self.data.len() && balance > 0 {
            let b = self.data[self.pos];
            match b {
                b'(' => {
                    balance += 1;
                    result.push(b);
                }
                b')' => {
                    balance -= 1;
                    if balance > 0 {
                        result.push(b);
                    }
                }
                b'\\' => {
                    self.pos += 1;
                    if self.pos < self.data.len() {
                        let b2 = self.data[self.pos];
                        match b2 {
                            b'n' => result.push(b'\n'),
                            b'r' => result.push(b'\r'),
                            b't' => result.push(b'\t'),
                            b'b' => result.push(8),
                            b'f' => result.push(12),
                            b'(' => result.push(b'('),
                            b')' => result.push(b')'),
                            b'\\' => result.push(b'\\'),
                            b'0'..=b'7' => {
                                let mut octal = (b2 - b'0') as u32;
                                let mut count = 1;
                                while count < 3 && self.pos + 1 < self.data.len() {
                                    let next_b = self.data[self.pos + 1];
                                    if (b'0'..=b'7').contains(&next_b) {
                                        octal = (octal << 3) | (next_b - b'0') as u32;
                                        self.pos += 1;
                                        count += 1;
                                    } else {
                                        break;
                                    }
                                }
                                result.push(octal as u8);
                            }
                            _ => result.push(b2),
                        }
                    }
                }
                _ => result.push(b),
            }
            self.pos += 1;
        }
        Ok(Token::String(Bytes::from(result)))
    }

    fn lex_hex_string(&mut self) -> PdfResult<Token> {
        self.pos += 1; // skip '<'
        let mut result = Vec::new();
        let mut high_nibble: Option<u8> = None;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b == b'>' {
                self.pos += 1;
                break;
            }
            if let Some(val) = (b as char).to_digit(16) {
                if let Some(high) = high_nibble {
                    result.push((high << 4) | val as u8);
                    high_nibble = None;
                } else {
                    high_nibble = Some(val as u8);
                }
            }
            self.pos += 1;
        }
        if let Some(high) = high_nibble {
            result.push(high << 4);
        }
        Ok(Token::Hex(Bytes::from(result)))
    }

    fn lex_number_or_keyword(&mut self) -> PdfResult<Token> {
        let start = self.pos;
        let mut is_real = false;
        while self.pos < self.data.len()
            && !is_delimiter(self.data[self.pos])
            && !is_whitespace(self.data[self.pos])
        {
            if self.data[self.pos] == b'.' {
                is_real = true;
            }
            self.pos += 1;
        }
        let s = String::from_utf8_lossy(&self.data[start..self.pos]);
        if is_real {
            if let Ok(f) = s.parse::<f64>() {
                return Ok(Token::Real(f));
            }
        } else if let Ok(i) = s.parse::<i64>() {
            return Ok(Token::Integer(i));
        }
        Ok(Token::Keyword(s.to_string()))
    }

    fn lex_keyword_or_other(&mut self) -> PdfResult<Token> {
        let start = self.pos;
        if self.pos < self.data.len() {
            self.pos += 1;
        }
        while self.pos < self.data.len()
            && !is_delimiter(self.data[self.pos])
            && !is_whitespace(self.data[self.pos])
        {
            self.pos += 1;
        }
        let s = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
        match s.as_str() {
            "true" => Ok(Token::Boolean(true)),
            "false" => Ok(Token::Boolean(false)),
            "null" => Ok(Token::Null),
            _ => Ok(Token::Keyword(s)),
        }
    }

    pub fn peek(&mut self) -> PdfResult<Token> {
        let prev_pos = self.pos;
        let token = self.next_token();
        self.pos = prev_pos;
        token
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}

fn is_newline(b: u8) -> bool {
    matches!(b, 10 | 13)
}

pub fn is_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}
