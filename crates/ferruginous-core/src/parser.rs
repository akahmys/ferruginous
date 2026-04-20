use crate::error::{PdfError, PdfResult};
use crate::lexer::{Lexer, Token};
use crate::types::{Object, PdfName, Reference, Resolver};
use bytes::Bytes;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct Parser<'a> {
    lexer: Lexer,
    peeked: Vec<Token>,
    depth: usize,
    resolver: Option<&'a dyn Resolver>,
}

const MAX_PARSE_DEPTH: usize = 64;

impl<'a> Parser<'a> {
    pub fn new(input: Bytes) -> Self {
        Self { lexer: Lexer::new(input), peeked: Vec::with_capacity(2), depth: 0, resolver: None }
    }

    pub fn with_resolver(mut self, resolver: &'a dyn Resolver) -> Self {
        self.resolver = Some(resolver);
        self
    }

    pub fn peek(&mut self) -> PdfResult<Option<&Token>> {
        self.peek_n(0)
    }

    pub fn peek_n(&mut self, n: usize) -> PdfResult<Option<&Token>> {
        while self.peeked.len() <= n {
            if let Some(t) = self.lexer.next_token()? {
                self.peeked.push(t);
            } else {
                return Ok(None);
            }
        }
        Ok(Some(&self.peeked[n]))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> PdfResult<Option<Token>> {
        if !self.peeked.is_empty() {
            Ok(Some(self.peeked.remove(0)))
        } else {
            self.lexer.next_token()
        }
    }

    pub fn parse_object(&mut self) -> PdfResult<Object> {
        if self.depth > MAX_PARSE_DEPTH {
            return Err(PdfError::Other("Max parse depth exceeded".into()));
        }
        self.depth += 1;
        let result = self.parse_object_internal();
        self.depth -= 1;
        result
    }

    fn parse_object_internal(&mut self) -> PdfResult<Object> {
        let t = self
            .next()?
            .ok_or_else(|| PdfError::Syntactic { pos: 0, message: "Unexpected EOF".into() })?;

        match t {
            Token::Keyword(b) if b.as_ref() == b"true" => Ok(Object::Boolean(true)),
            Token::Keyword(b) if b.as_ref() == b"false" => Ok(Object::Boolean(false)),
            Token::Keyword(b) if b.as_ref() == b"null" => Ok(Object::Null),
            Token::Integer(i) => {
                // Look ahead for reference: i [generation] R
                if let Ok(Some(Token::Integer(generation))) = self.peek() {
                    let generation = *generation;
                    if let Ok(Some(Token::Keyword(b))) = self.peek_n(1)
                        && b.as_ref() == b"R"
                    {
                        self.next()?; // consume generation
                        self.next()?; // consume R
                        return Ok(Object::Reference(Reference::new(i as u32, generation as u16)));
                    }
                }
                Ok(Object::Integer(i))
            }
            Token::Real(f) => Ok(Object::Real(f)),
            Token::LiteralString(s) => Ok(Object::String(s)),
            Token::HexString(s) => Ok(Object::String(s)),
            Token::Name(n) => Ok(Object::Name(PdfName(n))),
            Token::LeftSquareBracket => self.parse_array(),
            Token::DictionaryOpen => self.parse_dictionary_or_stream(),
            _ => Err(PdfError::Syntactic { pos: 0, message: format!("Unexpected token: {:?}", t) }),
        }
    }

    pub fn parse_indirect_object_header(&mut self) -> PdfResult<(u32, u16)> {
        let id_token = self
            .next()?
            .ok_or_else(|| PdfError::Syntactic { pos: 0, message: "Expected object ID".into() })?;
        let gen_token = self.next()?.ok_or_else(|| PdfError::Syntactic {
            pos: 0,
            message: "Expected generation number".into(),
        })?;

        let id = match id_token {
            Token::Integer(i) => i as u32,
            _ => {
                return Err(PdfError::Syntactic {
                    pos: 0,
                    message: format!("Expected integer object ID, found {:?}", id_token),
                });
            }
        };

        let generation = match gen_token {
            Token::Integer(i) => i as u16,
            _ => {
                return Err(PdfError::Syntactic {
                    pos: 0,
                    message: format!("Expected integer generation number, found {:?}", gen_token),
                });
            }
        };

        match self.next()? {
            Some(Token::Keyword(b)) if b.as_ref() == b"obj" => Ok((id, generation)),
            Some(t) => Err(PdfError::Syntactic {
                pos: 0,
                message: format!("Expected 'obj' keyword, found {:?}", t),
            }),
            None => Err(PdfError::Syntactic { pos: 0, message: "Unexpected EOF".into() }),
        }
    }

    fn parse_array(&mut self) -> PdfResult<Object> {
        let mut arr = Vec::new();
        loop {
            match self.peek()? {
                Some(Token::RightSquareBracket) => {
                    self.next()?;
                    break;
                }
                None => {
                    return Err(PdfError::Syntactic {
                        pos: 0,
                        message: "Unterminated array".into(),
                    });
                }
                _ => {
                    arr.push(self.parse_object()?);
                }
            }
        }
        Ok(Object::Array(Arc::new(arr)))
    }

    fn parse_dictionary_or_stream(&mut self) -> PdfResult<Object> {
        let mut dict = BTreeMap::new();
        loop {
            match self.peek()? {
                Some(Token::DictionaryClose) => {
                    self.next()?;
                    break;
                }
                None => {
                    return Err(PdfError::Syntactic {
                        pos: 0,
                        message: "Unterminated dictionary".into(),
                    });
                }
                Some(Token::Solidus) => {
                    self.next()?; // consume '/'
                }
                Some(Token::Name(_)) => {
                    let key = match self.next()? {
                        Some(Token::Name(n)) => PdfName(n),
                        _ => unreachable!("Peeked name but found other"),
                    };
                    let val = self.parse_object()?;
                    dict.insert(key, val);
                }
                _ => {
                    let t = self.next()?;
                    return Err(PdfError::Syntactic {
                        pos: 0,
                        message: format!("Expected key (Name) or >>, found {:?}", t),
                    });
                }
            }
        }

        if let Some(Token::Keyword(b)) = self.peek()?
            && b.as_ref() == b"stream"
        {
            self.next()?; // consume 'stream'
            return self.lex_stream_data(Arc::new(dict));
        }
        Ok(Object::Dictionary(Arc::new(dict)))
    }

    fn lex_stream_data(&mut self, dict: Arc<BTreeMap<PdfName, Object>>) -> PdfResult<Object> {
        let mut pos = 0;
        let input = self.lexer.input_remaining();

        // ISO 32000 says exactly \r\n or \n, but we allow flexible whitespace for robustness
        while pos < input.len()
            && (input[pos] == b'\r'
                || input[pos] == b'\n'
                || input[pos] == b' '
                || input[pos] == b'\t')
        {
            pos += 1;
        }
        self.lexer.advance(pos);

        // Resolve Length
        let length_obj = dict
            .get(&"Length".into())
            .ok_or_else(|| PdfError::Other("Missing /Length in stream".into()))?;
        let length = match length_obj {
            Object::Integer(i) => *i as usize,
            Object::Reference(r) => {
                if let Some(resolver) = self.resolver {
                    let resolved = resolver.resolve(r)?;
                    resolved
                        .as_i64()
                        .ok_or_else(|| PdfError::Other("Invalid /Length resolved".into()))?
                        as usize
                } else {
                    // Cannot resolve now, return empty stream (or placeholder)
                    // For now, let's keep it as 0 and let document fix it
                    0
                }
            }
            _ => return Err(PdfError::Other("Invalid /Length type".into())),
        };

        let stream_input = self.lexer.input_remaining();
        if length > stream_input.len() {
            return Err(PdfError::Other(format!(
                "Stream data too short (expected {}, got {})",
                length,
                stream_input.len()
            )));
        }

        // Zero-copy slice!
        // Wait, self.lexer.input is a Bytes. We need to slice it from the correct global position.
        // Actually Lexer::input handles the global slice.
        // Wait, Lexer::input is the WHOLE block?
        // Let's check how Lexer was initialized.

        // Actually, Lexer should expose a method to slice from its current position.
        // But for now, we know the length.

        // We'll need to update Lexer to support slicing correctly or just use the remaining input.
        // But Lexer::input is the shared Bytes.
        // lexer.pos is the current position.

        // I'll add a helper to Lexer for this.

        // Re-read Lexer implementation...

        let data = self.lexer.input_slice(length);
        self.lexer.advance(length);

        // Expect endstream
        self.lexer.skip_whitespace();
        match self.next()? {
            Some(Token::Keyword(b)) if b.as_ref() == b"endstream" => {}
            t => {
                return Err(PdfError::Syntactic {
                    pos: 0,
                    message: format!("Expected endstream, found {:?}", t),
                });
            }
        }

        Ok(Object::Stream(dict, data))
    }
}
