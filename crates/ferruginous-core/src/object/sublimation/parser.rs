//! Sublimation parser for PDF Content Streams.

use super::Command;
use crate::font::FontResource;
use crate::graphics::{Color, LineCap, LineJoin, StrokeStyle, TextRenderingMode, WindingRule};
use crate::lexer::{Lexer, Token};
use crate::object::{Object, PdfName};
use kurbo::{Affine, Point, Rect};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A stateful sublimator for converting raw content stream tokens into structured IR.
pub struct Sublimator<'a> {
    fonts: &'a BTreeMap<String, Arc<FontResource>>,
    stack: Vec<Token>,
    current_font: Option<Arc<FontResource>>,
}

impl<'a> Sublimator<'a> {
    pub fn new(fonts: &'a BTreeMap<String, Arc<FontResource>>) -> Self {
        Self { fonts, stack: Vec::new(), current_font: None }
    }

    pub fn sublimate(&mut self, data: &[u8]) -> Vec<Command> {
        let mut commands = Vec::new();
        let mut lexer = Lexer::new(bytes::Bytes::copy_from_slice(data));

        while let Ok(token) = lexer.next_token() {
            if token == Token::EOF {
                break;
            }

            match token {
                Token::Keyword(kw) => {
                    if let Some(cmd) = self.handle_operator(&kw) {
                        commands.push(cmd);
                    }
                }
                _ => self.stack.push(token),
            }
        }

        if let Some(res) = &self.current_font {
            commands.insert(0, Command::SetWritingMode(res.wmode as u8));
        }

        commands
    }

    fn handle_operator(&mut self, op: &str) -> Option<Command> {
        match op {
            "q" | "Q" | "cm" | "m" | "l" | "c" | "h" | "re" | "f" | "F" | "f*" | "S" | "s"
            | "B" | "B*" | "b" | "b*" | "W" | "W*" | "Do" => self.handle_graphics_op(op),
            "BT" | "ET" | "Tf" | "Tj" | "'" | "\"" | "TJ" | "Td" | "TD" | "Tm" | "Tc" | "Tw"
            | "Tz" | "Tr" | "Ts" => self.handle_text_op(op),
            "rg" | "RG" | "k" | "K" | "g" | "G" => self.handle_color_op(op),
            "BMC" | "BDC" | "EMC" => self.handle_marked_content_op(op),
            "d0" | "d1" => self.handle_type3_op(op),
            _ => {
                let operands = self.stack.drain(..).map(token_to_object).collect();
                Some(Command::RawOperator { name: op.to_string(), operands })
            }
        }
    }

    fn handle_graphics_op(&mut self, op: &str) -> Option<Command> {
        match op {
            "q" => Some(Command::PushState),
            "Q" => Some(Command::PopState),
            "cm" => self.pop_affine().map(Command::Transform),
            "m" => self.pop_point().map(Command::MoveTo),
            "l" => self.pop_point().map(Command::LineTo),
            "c" => self.pop_three_points().map(|(p1, p2, p3)| Command::CurveTo(p1, p2, p3)),
            "h" => Some(Command::ClosePath),
            "re" => self.pop_rect().map(Command::Rect),
            "f" | "F" => Some(Command::Fill(WindingRule::NonZero)),
            "f*" => Some(Command::Fill(WindingRule::EvenOdd)),
            "S" => self.create_stroke().map(Command::Stroke),
            "s" => {
                self.handle_operator("h");
                self.create_stroke().map(Command::Stroke)
            }
            "B" => self.create_stroke().map(|s| Command::FillStroke(WindingRule::NonZero, s)),
            "B*" => self.create_stroke().map(|s| Command::FillStroke(WindingRule::EvenOdd, s)),
            "b" => {
                self.handle_operator("h");
                self.create_stroke().map(|s| Command::FillStroke(WindingRule::NonZero, s))
            }
            "b*" => {
                self.handle_operator("h");
                self.create_stroke().map(|s| Command::FillStroke(WindingRule::EvenOdd, s))
            }
            "W" => Some(Command::Clip(WindingRule::NonZero)),
            "W*" => Some(Command::Clip(WindingRule::EvenOdd)),
            "Do" => self.pop_name().map(|n| Command::DrawXObject(n.as_str().to_string())),
            _ => None,
        }
    }

    fn handle_text_op(&mut self, op: &str) -> Option<Command> {
        match op {
            "BT" => Some(Command::BeginText),
            "ET" => Some(Command::EndText),
            "Tf" => self.handle_font_selection(),
            "Tj" | "'" | "\"" => self.handle_show_text(),
            "TJ" => self.handle_show_text_array(),
            "Td" => self.pop_point().map(Command::MoveText),
            "TD" => self.pop_point().map(Command::MoveText),
            "Tm" => self.pop_affine().map(Command::SetTextMatrix),
            "Tc" => self.pop_f64().map(Command::SetCharSpacing),
            "Tw" => self.pop_f64().map(Command::SetWordSpacing),
            "Tz" => self.pop_f64().map(Command::SetHorizontalScaling),
            "Tr" => self.pop_i64().map(|i| Command::SetTextRenderMode(TextRenderingMode::from(i))),
            "Ts" => self.pop_f64().map(Command::SetTextRise),
            _ => None,
        }
    }

    fn handle_color_op(&mut self, op: &str) -> Option<Command> {
        match op {
            "rg" => self.pop_rgb().map(Command::SetFillColor),
            "RG" => self.pop_rgb().map(Command::SetStrokeColor),
            "k" => self.pop_cmyk().map(Command::SetFillColor),
            "K" => self.pop_cmyk().map(Command::SetStrokeColor),
            "g" => self.pop_f64().map(|g| Command::SetFillColor(Color::Gray(g))),
            "G" => self.pop_f64().map(|g| Command::SetStrokeColor(Color::Gray(g))),
            _ => None,
        }
    }

    fn handle_marked_content_op(&mut self, op: &str) -> Option<Command> {
        match op {
            "BMC" => {
                self.pop_name().map(|n| Command::BeginMarkedContent { tag: n, properties: None })
            }
            "BDC" => self.handle_bdc(),
            "EMC" => Some(Command::EndMarkedContent),
            _ => None,
        }
    }

    fn handle_type3_op(&mut self, op: &str) -> Option<Command> {
        match op {
            "d0" => {
                let wy = self.pop_f64()?;
                let wx = self.pop_f64()?;
                Some(Command::Type3SetMetrics { wx, wy, bbox: None })
            }
            "d1" => {
                let ury = self.pop_f64()?;
                let urx = self.pop_f64()?;
                let lly = self.pop_f64()?;
                let llx = self.pop_f64()?;
                let wy = self.pop_f64()?;
                let wx = self.pop_f64()?;
                Some(Command::Type3SetMetrics {
                    wx,
                    wy,
                    bbox: Some(Rect::new(llx, lly, urx, ury)),
                })
            }
            _ => None,
        }
    }

    // --- Helpers for popping operands ---

    fn pop_f64(&mut self) -> Option<f64> {
        match self.stack.pop() {
            Some(Token::Real(f)) => Some(f),
            Some(Token::Integer(i)) => Some(i as f64),
            _ => None,
        }
    }

    fn pop_i64(&mut self) -> Option<i64> {
        match self.stack.pop() {
            Some(Token::Integer(i)) => Some(i),
            Some(Token::Real(f)) => Some(f as i64),
            _ => None,
        }
    }

    fn pop_point(&mut self) -> Option<Point> {
        let y = self.pop_f64()?;
        let x = self.pop_f64()?;
        Some(Point::new(x, y))
    }

    fn pop_three_points(&mut self) -> Option<(Point, Point, Point)> {
        let p3 = self.pop_point()?;
        let p2 = self.pop_point()?;
        let p1 = self.pop_point()?;
        Some((p1, p2, p3))
    }

    fn pop_rect(&mut self) -> Option<Rect> {
        let h = self.pop_f64()?;
        let w = self.pop_f64()?;
        let y = self.pop_f64()?;
        let x = self.pop_f64()?;
        Some(Rect::from_origin_size(Point::new(x, y), kurbo::Size::new(w, h)))
    }

    fn pop_affine(&mut self) -> Option<Affine> {
        let f = self.pop_f64()?;
        let e = self.pop_f64()?;
        let d = self.pop_f64()?;
        let c = self.pop_f64()?;
        let b = self.pop_f64()?;
        let a = self.pop_f64()?;
        Some(Affine::new([a, b, c, d, e, f]))
    }

    fn pop_rgb(&mut self) -> Option<Color> {
        let b = self.pop_f64()?;
        let g = self.pop_f64()?;
        let r = self.pop_f64()?;
        Some(Color::Rgb(r, g, b))
    }

    fn pop_cmyk(&mut self) -> Option<Color> {
        let k = self.pop_f64()?;
        let y = self.pop_f64()?;
        let m = self.pop_f64()?;
        let c = self.pop_f64()?;
        Some(Color::Cmyk(c, m, y, k))
    }

    fn pop_name(&mut self) -> Option<PdfName> {
        match self.stack.pop() {
            Some(Token::Name(b)) => Some(PdfName::from_bytes(&b)),
            _ => None,
        }
    }

    fn create_stroke(&self) -> Option<StrokeStyle> {
        Some(StrokeStyle {
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_pattern: None,
        })
    }

    fn handle_font_selection(&mut self) -> Option<Command> {
        let size = self.pop_f64()?;
        let name = self.pop_name()?;
        let name_str = name.as_str();

        if let Some(font_res) = self.fonts.get(name_str) {
            self.current_font = Some(font_res.clone());
            Some(Command::SetFont { font: name_str.to_string(), size })
        } else {
            None
        }
    }

    fn handle_show_text(&mut self) -> Option<Command> {
        let token = self.stack.pop()?;
        match token {
            Token::String(b) | Token::Hex(b) => Some(Command::ShowText(b)),
            _ => None,
        }
    }

    fn handle_show_text_array(&mut self) -> Option<Command> {
        let mut items = Vec::new();
        while let Some(t) = self.stack.pop() {
            if t == Token::LeftArray {
                break;
            }
            items.push(t);
        }
        items.reverse();

        let mut array_items = Vec::new();
        for t in items {
            match t {
                Token::String(b) | Token::Hex(b) => {
                    array_items.push(super::TextArrayItem::Text(b));
                }
                Token::Integer(i) => {
                    array_items.push(super::TextArrayItem::Offset(i as f64));
                }
                Token::Real(f) => {
                    array_items.push(super::TextArrayItem::Offset(f));
                }
                _ => {}
            }
        }
        Some(Command::ShowTextArray(array_items))
    }

    #[allow(dead_code)]
    fn decode_text_token(&self, token: &Token, font: &FontResource) -> String {
        let bytes = match token {
            Token::String(b) | Token::Hex(b) => b,
            _ => return String::new(),
        };

        let mut result = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let (consumed, unicode) = font.decode_next(&bytes[i..]);
            if consumed == 0 {
                break;
            }
            if let Some(u) = unicode {
                result.push_str(&u);
            }
            i += consumed;
        }
        result
    }

    fn handle_bdc(&mut self) -> Option<Command> {
        let _props = self.stack.pop();
        let tag = self.pop_name()?;
        Some(Command::BeginMarkedContent { tag, properties: None })
    }
}

fn token_to_object(token: Token) -> Object {
    match token {
        Token::Boolean(b) => Object::Boolean(b),
        Token::Integer(i) => Object::Integer(i),
        Token::Real(f) => Object::Real(f),
        Token::String(b) => Object::String(b),
        Token::Hex(b) => Object::Hex(b),
        Token::Name(_b) => Object::Name(crate::handle::Handle::new(0)), // Names are placeholders in IR operands
        Token::Null => Object::Null,
        _ => Object::Null,
    }
}
