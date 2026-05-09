//! Sublimation parser for PDF Content Streams.

use super::{Command, IrObject};
use crate::font::FontResource;
use crate::graphics::{Color, LineCap, LineJoin, StrokeStyle, TextRenderingMode, WindingRule};
use crate::lexer::{Lexer, Token};
use crate::object::PdfName;
use kurbo::{Affine, Point, Rect};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A stateful sublimator for converting raw content stream tokens into structured IR.
pub struct Sublimator<'a> {
    fonts: &'a BTreeMap<String, Arc<FontResource>>,
    stack: Vec<IrObject>,
    current_font: Option<Arc<FontResource>>,
}

impl<'a> Sublimator<'a> {
    pub fn new(fonts: &'a BTreeMap<String, Arc<FontResource>>) -> Self {
        Self { fonts, stack: Vec::new(), current_font: None }
    }

    pub fn sublimate(&mut self, data: &[u8]) -> Vec<Command> {
        // DETECT CORRUPTION: If the stream looks like Rust debug output, attempt resurrection (ISO 32000-2:2020 Clause 7.8.2 Fallback)
        if (data.starts_with(b"PushState") || data.starts_with(b"RawOperator"))
            && let Some(cmds) = super::resurrection::resurrect_commands(data) {
                return cmds;
            }

        let mut commands = Vec::new();
        let mut lexer = Lexer::new(bytes::Bytes::copy_from_slice(data));

        while let Ok(token) = lexer.next_token() {
            if token == Token::EOF {
                break;
            }

            match token {
                Token::Keyword(kw) => {
                    let mut cmds = self.handle_operator(&kw);
                    commands.append(&mut cmds);
                }
                Token::LeftArray => {
                    let arr = self.parse_ir_array(&mut lexer);
                    self.stack.push(arr);
                }
                Token::LeftDict => {
                    let dict = self.parse_ir_dict(&mut lexer);
                    self.stack.push(dict);
                }
                _ => {
                    if let Some(ir) = token_to_ir_object(token) {
                        self.stack.push(ir);
                    }
                }
            }
        }

        commands
    }

    fn handle_operator(&mut self, op: &str) -> Vec<Command> {
        let cmd_opt = match op {
            "q" | "Q" | "cm" | "m" | "l" | "c" | "h" | "re" | "f" | "F" | "f*" | "S" | "s"
            | "B" | "B*" | "b" | "b*" | "W" | "W*" | "Do" | "w" | "J" | "j" | "M" | "d" | "i" => {
                self.handle_graphics_op(op)
            }
            "BT" | "ET" | "Tf" | "Tj" | "'" | "\"" | "TJ" | "Td" | "TD" | "Tm" | "Tc" | "Tw"
            | "Tz" | "Tr" | "Ts" | "TL" | "T*" => return self.handle_text_op(op),
            "rg" | "RG" | "k" | "K" | "g" | "G" | "cs" | "CS" | "scn" | "SCN" | "sc" | "SC" => {
                self.handle_color_op(op)
            }
            "BMC" | "BDC" | "EMC" => self.handle_marked_content_op(op),
            "d0" | "d1" => self.handle_type3_op(op),
            _ => None,
        };

        if let Some(cmd) = cmd_opt {
            vec![cmd]
        } else {
            // Drain the stack and emit a RawOperator to prevent corruption of subsequent commands
            let operands = self.stack.drain(..).collect();
            log::warn!("[SUBLIMATE] Emitting RawOperator for {}: {:?}", op, operands);
            vec![Command::RawOperator { name: op.to_string(), operands }]
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
            "w" | "J" | "j" | "M" | "d" | "i" => {
                let operands = self.stack.drain(..).collect();
                Some(Command::RawOperator { name: op.to_string(), operands })
            }
            _ => None,
        }
    }

    fn handle_text_op(&mut self, op: &str) -> Vec<Command> {
        match op {
            "BT" => vec![Command::BeginText],
            "ET" => vec![Command::EndText],
            "Tf" => self.handle_font_selection(),
            "Tj" => self.handle_show_text().into_iter().collect(),
            "'" => self.handle_quote_op(),
            "\"" => self.handle_double_quote_op(),
            "TJ" => self.handle_show_text_array().into_iter().collect(),
            "Td" => self.pop_point().map(Command::MoveText).into_iter().collect(),
            "TD" => self.handle_td_op(),
            "Tm" => self.pop_affine().map(Command::SetTextMatrix).into_iter().collect(),
            "Tc" => self.pop_f64().map(Command::SetCharSpacing).into_iter().collect(),
            "Tw" => self.pop_f64().map(Command::SetWordSpacing).into_iter().collect(),
            "Tz" => self.pop_f64().map(Command::SetHorizontalScaling).into_iter().collect(),
            "Tr" => self
                .pop_i64()
                .map(|i| Command::SetTextRenderMode(TextRenderingMode::from(i)))
                .into_iter()
                .collect(),
            "Ts" => self.pop_f64().map(Command::SetTextRise).into_iter().collect(),
            "TL" => self.pop_f64().map(Command::SetTextLeading).into_iter().collect(),
            "T*" => vec![Command::MoveToNextLine],
            _ => Vec::new(),
        }
    }

    fn handle_quote_op(&mut self) -> Vec<Command> {
        let mut cmds = vec![Command::MoveToNextLine];
        if let Some(text) = self.handle_show_text() {
            cmds.push(text);
        }
        cmds
    }

    fn handle_double_quote_op(&mut self) -> Vec<Command> {
        let mut cmds = Vec::new();
        let string = self.stack.pop();
        let char_spacing = self.pop_f64();
        let word_spacing = self.pop_f64();

        if let Some(w) = word_spacing {
            cmds.push(Command::SetWordSpacing(w));
        }
        if let Some(c) = char_spacing {
            cmds.push(Command::SetCharSpacing(c));
        }
        cmds.push(Command::MoveToNextLine);
        if let Some(token) = string {
            self.stack.push(token);
            if let Some(text) = self.handle_show_text() {
                cmds.push(text);
            }
        }
        cmds
    }

    fn handle_td_op(&mut self) -> Vec<Command> {
        if let Some(p) = self.pop_point() {
            vec![Command::SetTextLeading(-p.y), Command::MoveText(p)]
        } else {
            Vec::new()
        }
    }

    fn handle_color_op(&mut self, op: &str) -> Option<Command> {
        match op {
            "rg" => self.pop_rgb().map(|c| Command::SetFillColor(c.to_rgb())),
            "RG" => self.pop_rgb().map(|c| Command::SetStrokeColor(c.to_rgb())),
            "k" => self.pop_cmyk().map(|c| Command::SetFillColor(c.to_rgb())),
            "K" => self.pop_cmyk().map(|c| Command::SetStrokeColor(c.to_rgb())),
            "g" => self.pop_f64().map(|g| Command::SetFillColor(Color::Gray(g).to_rgb())),
            "G" => self.pop_f64().map(|g| Command::SetStrokeColor(Color::Gray(g).to_rgb())),
            "cs" | "CS" | "scn" | "SCN" | "sc" | "SC" => {
                // STUB: Consume operands and return as Raw for now, or implement basic sc/SC
                let operands = self.stack.drain(..).collect();
                Some(Command::RawOperator { name: op.to_string(), operands })
            }
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
                Some(Command::Type3SetMetrics { wx, wy, bbox: Some(Rect::new(llx, lly, urx, ury)) })
            }
            _ => None,
        }
    }

    // --- Helpers for popping operands ---

    fn pop_f64(&mut self) -> Option<f64> {
        match self.stack.pop() {
            Some(IrObject::Real(f)) => Some(f),
            Some(IrObject::Integer(i)) => Some(i as f64),
            _ => None,
        }
    }

    fn pop_i64(&mut self) -> Option<i64> {
        match self.stack.pop() {
            Some(IrObject::Integer(i)) => Some(i),
            Some(IrObject::Real(f)) => Some(f as i64),
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
            Some(IrObject::Name(s)) => Some(PdfName::new(&s)),
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

    fn handle_font_selection(&mut self) -> Vec<Command> {
        let Some(size) = self.pop_f64() else { return Vec::new() };
        let Some(name) = self.pop_name() else { return Vec::new() };
        let name_str = name.as_str();

        if let Some(font_res) = self.fonts.get(name_str) {
            self.current_font = Some(font_res.clone());
            vec![
                Command::SetFont { font: name_str.to_string(), size },
                Command::SetWritingMode(font_res.wmode),
            ]
        } else {
            log::error!("[SUBLIMATE] Font /{} not found in resources! Available: {:?}", name_str, self.fonts.keys().collect::<Vec<_>>());
            // Insert a SetFont command anyway, but mark it for fallback resolution in SDK
            vec![Command::SetFont { font: "Fallback-Sans".to_string(), size }]
        }
    }

    fn handle_show_text(&mut self) -> Option<Command> {
        let obj = self.stack.pop()?;
        match obj {
            IrObject::String(b) | IrObject::Hex(b) => Some(Command::ShowText(b)),
            _ => None,
        }
    }

    fn handle_show_text_array(&mut self) -> Option<Command> {
        let obj = self.stack.pop()?;
        let items = match obj {
            IrObject::Array(arr) => arr,
            _ => return None,
        };

        let mut array_items = Vec::new();
        for t in items {
            match t {
                IrObject::String(b) | IrObject::Hex(b) => {
                    array_items.push(super::TextArrayItem::Text(b));
                }
                IrObject::Integer(i) => {
                    array_items.push(super::TextArrayItem::Offset(i as f64));
                }
                IrObject::Real(f) => {
                    array_items.push(super::TextArrayItem::Offset(f));
                }
                _ => {}
            }
        }
        Some(Command::ShowTextArray(array_items))
    }


    fn handle_bdc(&mut self) -> Option<Command> {
        let props = self.stack.pop();
        let tag = self.pop_name()?;
        Some(Command::BeginMarkedContent { tag, properties: props })
    }

    fn parse_ir_array(&self, lexer: &mut Lexer) -> IrObject {
        let mut elements = Vec::new();
        while let Ok(token) = lexer.peek() {
            if token == Token::RightArray || token == Token::EOF {
                break;
            }
            let _ = lexer.next_token();
            match token {
                Token::LeftArray => elements.push(self.parse_ir_array(lexer)),
                Token::LeftDict => elements.push(self.parse_ir_dict(lexer)),
                _ => {
                    if let Some(ir) = token_to_ir_object(token) {
                        elements.push(ir);
                    }
                }
            }
        }
        let _ = lexer.next_token(); // consume ']'
        IrObject::Array(elements)
    }

    fn parse_ir_dict(&self, lexer: &mut Lexer) -> IrObject {
        let mut dict = BTreeMap::new();
        while let Ok(token) = lexer.peek() {
            if token == Token::RightDict || token == Token::EOF {
                break;
            }
            let key_token = lexer.next_token().unwrap();
            let key = match key_token {
                Token::Name(b) => crate::refine::text::recover_string(&b),
                _ => continue, // Should be an error but let's be robust
            };

            let val_token = lexer.next_token().unwrap();
            let val = match val_token {
                Token::LeftArray => self.parse_ir_array(lexer),
                Token::LeftDict => self.parse_ir_dict(lexer),
                _ => token_to_ir_object(val_token).unwrap_or(IrObject::Null),
            };
            dict.insert(key, val);
        }
        let _ = lexer.next_token(); // consume '>>'
        IrObject::Dictionary(dict)
    }
}

fn token_to_ir_object(token: Token) -> Option<IrObject> {
    match token {
        Token::Boolean(b) => Some(IrObject::Boolean(b)),
        Token::Integer(i) => Some(IrObject::Integer(i)),
        Token::Real(f) => Some(IrObject::Real(f)),
        Token::String(s) => Some(IrObject::String(s)),
        Token::Hex(s) => Some(IrObject::Hex(s)),
        Token::Name(n) => Some(IrObject::Name(crate::refine::text::recover_string(&n))),
        Token::Null => Some(IrObject::Null),
        _ => None,
    }
}
