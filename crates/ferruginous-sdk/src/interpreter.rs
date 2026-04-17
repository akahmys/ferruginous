use ferruginous_core::{Object, Parser, PdfResult, PdfError, lexer::Token, PdfName};
use ferruginous_render::{RenderBackend, path::PathBuilder};
use ferruginous_core::graphics::{Color, WindingRule, StrokeStyle};

/// A content stream interpreter that translates PDF operators into [RenderBackend] calls.
///
/// It maintains a graphics stack and a path builder to handle stateful
/// PDF rendering operations.
pub struct Interpreter<'a> {
    backend: &'a mut dyn RenderBackend,
    stack: Vec<Object>,
    path: PathBuilder,
}

impl<'a> Interpreter<'a> {
    /// Creates a new interpreter tied to a specific rendering backend.
    pub fn new(backend: &'a mut dyn RenderBackend) -> Self {
        Self {
            backend,
            stack: Vec::new(),
            path: PathBuilder::new(),
        }
    }

    /// Executes a content stream by parsing and processing its operators.
    pub fn execute(&mut self, data: &[u8]) -> PdfResult<()> {
        let mut parser = Parser::new(bytes::Bytes::copy_from_slice(data));

        while let Some(token) = parser.next()? {
            match token {
                Token::Keyword(s) if s.as_ref() == b"true" => self.stack.push(Object::Boolean(true)),
                Token::Keyword(s) if s.as_ref() == b"false" => self.stack.push(Object::Boolean(false)),
                Token::Keyword(op) => {
                    let s = std::str::from_utf8(op.as_ref()).unwrap_or("");
                    self.execute_operator(s)?;
                }
                Token::Integer(i) => self.stack.push(Object::Integer(i)),
                Token::Real(f) => self.stack.push(Object::Real(f)),
                Token::Name(n) => self.stack.push(Object::Name(PdfName(n))),
                Token::LiteralString(s) => self.stack.push(Object::String(s)),
                Token::HexString(s) => self.stack.push(Object::String(s)),
                _ => {}
            }
        }
        Ok(())
    }

    #[allow(clippy::many_single_char_names)]
    fn execute_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            // Path Construction
            "m" => {
                let y = self.pop_f64()?;
                let x = self.pop_f64()?;
                self.path.move_to(x, y);
            }
            "l" => {
                let y = self.pop_f64()?;
                let x = self.pop_f64()?;
                self.path.line_to(x, y);
            }
            "c" => {
                let y3 = self.pop_f64()?; let x3 = self.pop_f64()?;
                let y2 = self.pop_f64()?; let x2 = self.pop_f64()?;
                let y1 = self.pop_f64()?; let x1 = self.pop_f64()?;
                self.path.curve_to(x1, y1, x2, y2, x3, y3);
            }
            "re" => {
                let h = self.pop_f64()?; let w = self.pop_f64()?;
                let y = self.pop_f64()?; let x = self.pop_f64()?;
                self.path.rectangle(x, y, w, h);
            }
            "h" => {
                self.path.close_path();
            }

            // Path Painting
            "S" => {
                let p = std::mem::replace(&mut self.path, PathBuilder::new()).finish();
                self.backend.stroke_path(&p, &Color::Gray(0.0), &StrokeStyle::default());
            }
            "f" | "F" => {
                let p = std::mem::replace(&mut self.path, PathBuilder::new()).finish();
                self.backend.fill_path(&p, &Color::Gray(0.0), WindingRule::NonZero);
            }
            "f*" => {
                let p = std::mem::replace(&mut self.path, PathBuilder::new()).finish();
                self.backend.fill_path(&p, &Color::Gray(0.0), WindingRule::EvenOdd);
            }

            // Graphics State
            "q" => self.backend.push_state(),
            "Q" => self.backend.pop_state(),
            "cm" => {
                let f = self.pop_f64()?; let e = self.pop_f64()?;
                let d = self.pop_f64()?; let c = self.pop_f64()?;
                let b = self.pop_f64()?; let a = self.pop_f64()?;
                self.backend.transform(kurbo::Affine::new([a, b, c, d, e, f]));
            }

            _ => {
                // Ignore unknown operators for now
            }
        }
        self.stack.clear();
        Ok(())
    }

    fn pop_f64(&mut self) -> PdfResult<f64> {
        match self.stack.pop() {
            Some(Object::Real(f)) => Ok(f),
            Some(Object::Integer(i)) => Ok(i as f64),
            _ => Err(PdfError::Other("Expected number".into())),
        }
    }
}
