//! ISO 32000-2:2020 Clause 7.3 - Objects

use crate::PdfResult;
use crate::arena::PdfArena;
use crate::error::PdfError;
use crate::handle::Handle;
use crate::lexer::{Lexer, Token};
use crate::object::{Object, PdfName};
use bytes::Bytes;
use std::collections::BTreeMap;

pub struct Parser<'a> {
    lexer: Lexer,
    arena: &'a PdfArena,
}

impl<'a> Parser<'a> {
    pub fn new(data: Bytes, arena: &'a PdfArena) -> Self {
        Self { lexer: Lexer::new(data), arena }
    }

    pub fn peek(&mut self) -> PdfResult<Token> {
        self.lexer.peek()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> PdfResult<Token> {
        self.lexer.next()
    }

    /// Parses a single PDF object from the token stream.
    pub fn parse_object(&mut self) -> PdfResult<Object> {
        let token = self.lexer.next()?;
        match token {
            Token::Boolean(b) => Ok(Object::Boolean(b)),
            Token::Integer(i) => {
                // Peek to see if it's the start of an indirect reference (R)
                let saved_pos = self.lexer.pos();
                if let Ok(Token::Integer(_gen_num)) = self.lexer.next()
                    && let Ok(Token::Keyword(ref k)) = self.lexer.peek()
                    && k == "R"
                {
                    let _ = self.lexer.next(); // consume "R"
                    return Ok(Object::Reference(Handle::new(i as u32)));
                }
                // Backtrack if it's not an indirect reference
                self.lexer.set_pos(saved_pos);
                Ok(Object::Integer(i))
            }
            Token::Real(f) => Ok(Object::Real(f)),
            Token::String(s) => Ok(Object::String(s)),
            Token::Name(n) => {
                let handle = self.arena.intern_name(PdfName::new(&n));
                Ok(Object::Name(handle))
            }
            Token::Null => Ok(Object::Null),
            Token::LeftArray => self.parse_array(),
            Token::LeftDict => self.parse_dict(),
            Token::EOF => Err(PdfError::Parse("Unexpected EOF".into())),
            _ => Err(PdfError::Parse(format!("Unexpected token: {:?}", token))),
        }
    }

    fn parse_array(&mut self) -> PdfResult<Object> {
        let mut elements = Vec::new();
        while self.lexer.peek()? != Token::RightArray && self.lexer.peek()? != Token::EOF {
            elements.push(self.parse_object()?);
        }
        self.lexer.next()?; // consume ']'
        let handle = self.arena.alloc_array(elements);
        Ok(Object::Array(handle))
    }

    fn parse_dict(&mut self) -> PdfResult<Object> {
        let mut dict = BTreeMap::new();
        while self.lexer.peek()? != Token::RightDict && self.lexer.peek()? != Token::EOF {
            let key_token = self.lexer.next()?;
            let key_handle = match key_token {
                Token::Name(n) => self.arena.intern_name(PdfName::new(&n)),
                _ => {
                    return Err(PdfError::Parse(format!(
                        "Expected name as dictionary key, found {:?}",
                        key_token
                    )));
                }
            };
            let val = self.parse_object()?;
            dict.insert(key_handle, val);
        }
        self.lexer.next()?; // consume '>>'

        let handle = self.arena.alloc_dict(dict);
        Ok(Object::Dictionary(handle))
    }
}
