//! ISO 32000-2:2020 Clause 7.2 - Lexical Conventions

use crate::PdfResult;
use bytes::Bytes;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(Bytes),
    Name(String),
    Keyword(String),
    LeftArray,
    RightArray,
    LeftDict,
    RightDict,
    Comment(String),
    Null,
    EOF,
}

pub struct Lexer {
    data: Bytes,
    pos: usize,
}

impl Lexer {
    pub fn new(data: Bytes) -> Self {
        Self { data, pos: 0 }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> PdfResult<Token> {
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
        let start = self.pos;
        while self.pos < self.data.len()
            && !is_delimiter(self.data[self.pos])
            && !is_whitespace(self.data[self.pos])
        {
            self.pos += 1;
        }
        let name = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
        Ok(Token::Name(name))
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
                            b'(' | b')' | b'\\' => result.push(b2),
                            b'\r' | b'\n' => { /* ignore line break */ }
                            _ => {
                                /* handle octal if needed, but simplified for now */
                                result.push(b2);
                            }
                        }
                    }
                }
                _ => {
                    result.push(b);
                }
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
        Ok(Token::String(Bytes::from(result)))
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
        // Ensure we advance at least one byte if we're not at EOF and not a known delimiter
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
        let token = self.next();
        self.pos = prev_pos;
        token
    }

    /// Returns the current byte position in the stream.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// sets the current byte position in the stream (use with caution).
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

fn is_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}
