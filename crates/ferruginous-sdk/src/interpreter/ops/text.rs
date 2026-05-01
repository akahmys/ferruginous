use crate::interpreter::Interpreter;
use ferruginous_core::font::FontResource;
use ferruginous_core::graphics::{Matrix, TextMatrices};
use ferruginous_core::{Handle, Object, PdfError, PdfResult};

use ferruginous_core::object::sublimation::Command;

impl Interpreter<'_> {
    /// Dispatches a normalized text command to the appropriate operator handler.
    pub(crate) fn handle_text_command(&mut self, cmd: &Command) -> PdfResult<()> {
        match cmd {
            Command::BeginText => self.handle_text_scope_operator("BT"),
            Command::EndText => self.handle_text_scope_operator("ET"),
            Command::ShowText(s) => self.show_text(s),
            Command::ShowTextArray(arr) => self.handle_show_text_array_ir(arr),
            Command::SetFont { font, size } => {
                let name = self.doc.arena().intern_name(ferruginous_core::PdfName::new(font));
                self.stack.push(Object::Name(name));
                self.stack.push(Object::Real(*size));
                self.handle_text_state_operator("Tf")
            }
            Command::MoveText(p) => {
                self.stack.push(Object::Real(p.x));
                self.stack.push(Object::Real(p.y));
                self.handle_text_positioning_operator("Td")
            }
            Command::SetTextMatrix(m) => {
                let c = m.as_coeffs();
                for coeff in &c {
                    self.stack.push(Object::Real(*coeff));
                }
                self.handle_text_positioning_operator("Tm")
            }
            Command::SetTextRise(f) => {
                self.stack.push(Object::Real(*f));
                self.handle_text_state_operator("Ts")
            }
            Command::SetCharSpacing(s) => {
                self.stack.push(Object::Real(*s));
                self.handle_text_state_operator("Tc")
            }
            Command::SetWordSpacing(s) => {
                self.stack.push(Object::Real(*s));
                self.handle_text_state_operator("Tw")
            }
            Command::SetHorizontalScaling(s) => {
                self.stack.push(Object::Real(*s));
                self.handle_text_state_operator("Tz")
            }
            Command::SetTextRenderMode(m) => {
                self.stack.push(Object::Integer(*m as i64));
                self.handle_text_state_operator("Tr")
            }
            Command::SetWritingMode(w) => {
                self.state.text_state.wmode = *w;
                Ok(())
            }
            Command::Type3SetMetrics { .. } => {
                // ISO 32000-2 9.6.5: d0/d1 set the glyph width.
                // We currently handle width in show_text, so this is a no-op here.
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Handles operators that manage the scope of a text object (BT, ET).
    pub(crate) fn handle_text_scope_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "BT" => {
                self.text_matrices = Some(TextMatrices::default());
            }
            "ET" => {
                self.text_matrices = None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles operators that modify the text state (Tf, Ts, Tc, Tw, Tz, Tr).
    pub(crate) fn handle_text_state_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Tf" => {
                let size = self.pop_f64()?;
                let name = self.pop_name()?;
                self.state.text_state.font = Some(name.clone());
                self.state.text_state.font_size = size;
                let _ = self.resolve_font_resource(&name)?;
            }
            "Ts" => {
                self.state.text_state.rise = self.pop_f64()?;
            }
            "Tc" => {
                let spacing = self.pop_f64()?;
                self.state.text_state.char_spacing = spacing;
                self.backend.set_char_spacing(spacing);
            }
            "Tw" => {
                let spacing = self.pop_f64()?;
                self.state.text_state.word_spacing = spacing;
                self.backend.set_word_spacing(spacing);
            }
            "Tz" => {
                self.state.text_state.horizontal_scaling = self.pop_f64()?;
            }
            "Tr" => {
                let mode = self.pop_i64()?;
                let m = ferruginous_core::graphics::TextRenderingMode::from(mode);
                self.state.text_state.rendering_mode = m;
                self.backend.set_text_render_mode(m);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles operators that position the text (Td, TD, Tm, T*).
    pub(crate) fn handle_text_positioning_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Td" => {
                let ty = self.pop_f64()?;
                let tx = self.pop_f64()?;
                let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
                let nl = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                m.tlm = m.tlm.concat(&nl);
                m.tm = m.tlm;
            }
            "TD" => {
                let ty = self.pop_f64()?;
                let tx = self.pop_f64()?;
                self.state.text_state.leading = -ty;
                let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
                let nl = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                m.tlm = m.tlm.concat(&nl);
                m.tm = m.tlm;
            }
            "Tm" => {
                let f = self.pop_f64()?;
                let e = self.pop_f64()?;
                let d = self.pop_f64()?;
                let c = self.pop_f64()?;
                let b = self.pop_f64()?;
                let a = self.pop_f64()?;
                let mat = Matrix::new(a, b, c, d, e, f);
                let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
                m.tlm = mat;
                m.tm = mat;
            }
            "T*" => {
                let leading = self.state.text_state.leading;
                let font_name = self.state.text_state.font.clone();
                let is_vertical = if let Some(ref f) = font_name {
                    self.resolve_font_resource(f).map(|r| r.wmode() == 1).unwrap_or(false)
                } else {
                    false
                };

                let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
                let nl = if is_vertical {
                    Matrix::new(1.0, 0.0, 0.0, 1.0, -leading, 0.0)
                } else {
                    Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -leading)
                };
                m.tlm = m.tlm.concat(&nl);
                m.tm = m.tlm;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles operators that show text (Tj, TJ, ', ").
    pub(crate) fn handle_text_showing_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Tj" => {
                let s = self.pop_string()?;
                self.show_text(&s)?;
            }
            "TJ" => {
                let a = self.pop_array()?;
                self.show_text_array(a)?;
            }
            "'" => {
                self.handle_text_positioning_operator("T*")?;
                let s = self.pop_string()?;
                self.show_text(&s)?;
            }
            "\"" => {
                let s = self.pop_string()?;
                let tc = self.pop_f64()?;
                let tw = self.pop_f64()?;
                self.state.text_state.word_spacing = tw;
                self.backend.set_word_spacing(tw);
                self.state.text_state.char_spacing = tc;
                self.backend.set_char_spacing(tc);
                self.stack.push(Object::String(s));
                self.handle_text_showing_operator("'")?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles the execution of a pre-sublimated text array command (TJ).
    ///
    /// This method preserves numeric offsets for kerning and precise character positioning,
    /// ensuring correct layout for complex vertical and horizontal text blocks.
    pub(crate) fn handle_show_text_array_ir(
        &mut self,
        arr: &[ferruginous_core::object::sublimation::TextArrayItem],
    ) -> PdfResult<()> {
        use ferruginous_core::object::sublimation::TextArrayItem;

        let font_name = self.state.text_state.font.clone();
        let wmode = if let Some(ref f) = font_name {
            self.resolve_font_resource(f).map(|r| r.wmode()).unwrap_or(0)
        } else {
            0
        };

        for item in arr {
            match item {
                TextArrayItem::Text(s) => self.show_text(s)?,
                TextArrayItem::Offset(n) => {
                    let th = self.state.text_state.horizontal_scaling / 100.0;
                    let displacement = n / 1000.0 * self.state.text_state.font_size;
                    let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
                    let shift = if wmode == 1 {
                        Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -displacement)
                    } else {
                        Matrix::new(1.0, 0.0, 0.0, 1.0, -displacement * th, 0.0)
                    };
                    m.tm = m.tm.concat(&shift);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn show_text(&mut self, text: &[u8]) -> PdfResult<()> {
        let name =
            self.state.text_state.font.clone().ok_or_else(|| PdfError::Other("No font".into()))?;
        let res = self.resolve_font_resource(&name)?;
        let glyphs = self.map_text_to_glyphs(text, &res)?;

        let tm = self.text_matrices.as_ref().map(|m| m.tm).unwrap_or_default();
        let rise_mat = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, self.state.text_state.rise);
        let render = tm.concat(&rise_mat);

        let th = self.state.text_state.horizontal_scaling / 100.0;
        let text_state = ferruginous_render::TextState {
            th,
            tc: self.state.text_state.char_spacing,
            tw: self.state.text_state.word_spacing,
            is_vertical: res.wmode() == 1,
        };

        if let Some(char_procs) = res.char_procs.clone() {
            self.render_type3_glyphs(&glyphs, &res, &render, &text_state, &char_procs)?;
        } else {
            self.backend.show_text(
                &glyphs,
                self.state.text_state.font_size,
                render.as_affine(),
                text_state,
                self.op_index,
            );
        }

        // Calculate total advance to update text matrix
        let advance_mat = self.calculate_text_advance(&glyphs, res.wmode() == 1, th);
        let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
        m.tm = m.tm.concat(&advance_mat);
        Ok(())
    }

    fn render_type3_glyphs(
        &mut self,
        glyphs: &[ferruginous_render::TextGlyph],
        font: &FontResource,
        render: &Matrix,
        text_state: &ferruginous_render::TextState,
        char_procs: &std::collections::BTreeMap<String, Handle<Object>>,
    ) -> PdfResult<()> {
        let font_matrix = font.font_matrix.unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);
        let fm = kurbo::Affine::new(font_matrix.map(|v| v as f64));
        let mut current_advance = 0.0;

        for glyph in glyphs {
            if let Some(name) = &glyph.name {
                let clean_name = name.strip_prefix('/').unwrap_or(name);
                if let Some(stream_h) = char_procs.get(clean_name) {
                    self.backend.push_state();
                    let adv_vec = if text_state.is_vertical {
                        kurbo::Vec2::new(0.0, current_advance)
                    } else {
                        kurbo::Vec2::new(current_advance, 0.0)
                    };

                    let size = self.state.text_state.font_size;
                    let h_scale = if text_state.is_vertical { 1.0 } else { text_state.th };
                    let local_to_pt = kurbo::Affine::scale_non_uniform(size * h_scale, size) * fm;

                    let t = render.as_affine() * kurbo::Affine::translate(adv_vec) * local_to_pt;
                    self.backend.set_transform(self.state.ctm.as_affine() * t);

                    // Execute glyph stream
                    let _ = self.execute(*stream_h);

                    self.backend.pop_state();
                }
            }

            // Calculate advance for next glyph
            let mut advance = f64::from(glyph.width) / 1000.0 * self.state.text_state.font_size;
            if !text_state.is_vertical {
                advance *= text_state.th;
                advance += text_state.tc * text_state.th;
                if glyph.char_code == 0x20 {
                    advance += text_state.tw * text_state.th;
                }
            } else {
                advance -= text_state.tc;
                if glyph.char_code == 0x20 {
                    advance -= text_state.tw;
                }
            }
            current_advance += advance;
        }
        Ok(())
    }

    fn calculate_text_advance(
        &self,
        glyphs: &[ferruginous_render::TextGlyph],
        is_vertical: bool,
        th: f64,
    ) -> Matrix {
        let mut total_advance = 0.0;
        let font_size = self.state.text_state.font_size;
        let tc = self.state.text_state.char_spacing;
        let tw = self.state.text_state.word_spacing;

        for glyph in glyphs {
            let char_width = f64::from(glyph.width) / 1000.0 * font_size;
            if is_vertical {
                total_advance += char_width - tc;
                if glyph.char_code == 0x20 {
                    total_advance -= tw;
                }
            } else {
                total_advance += (char_width + tc) * th;
                if glyph.char_code == 0x20 {
                    total_advance += tw * th;
                }
            }
        }

        if is_vertical {
            Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, total_advance)
        } else {
            Matrix::new(1.0, 0.0, 0.0, 1.0, total_advance, 0.0)
        }
    }

    pub(crate) fn show_text_array(&mut self, arr: Handle<Vec<Object>>) -> PdfResult<()> {
        if let Some(array) = self.doc.arena().get_array(arr) {
            let font_name = self.state.text_state.font.clone();
            let wmode = if let Some(ref f) = font_name {
                self.resolve_font_resource(f).map(|r| r.wmode()).unwrap_or(0)
            } else {
                0
            };

            for obj in array {
                match obj {
                    Object::String(s) => self.show_text(&s)?,
                    Object::Hex(s) => self.show_text(&s)?,
                    Object::Text(s) => self.show_text(s.as_bytes())?,
                    _ if obj.as_f64().is_some() => {
                        let n = obj
                            .as_f64()
                            .ok_or_else(|| PdfError::Other("Invalid number in TJ".into()))?;
                        let th = self.state.text_state.horizontal_scaling / 100.0;
                        let displacement = n / 1000.0 * self.state.text_state.font_size;
                        let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
                        let shift = if wmode == 1 {
                            Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -displacement)
                        } else {
                            Matrix::new(1.0, 0.0, 0.0, 1.0, -displacement * th, 0.0)
                        };
                        m.tm = m.tm.concat(&shift);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub(crate) fn map_text_to_glyphs(
        &self,
        text: &[u8],
        font: &FontResource,
    ) -> PdfResult<Vec<ferruginous_render::TextGlyph>> {
        let mut glyphs = Vec::new();
        let mut i = 0;
        while i < text.len() {
            let (consumed, u) = font.decode_next(&text[i..]);
            if consumed == 0 {
                break;
            }
            if i + consumed > text.len() {
                break;
            }
            let code = &text[i..i + consumed];
            let cid = font.to_cid(code);
            let char_code = if consumed == 1 {
                u32::from(code[0])
            } else if consumed == 2 {
                (u32::from(code[0]) << 8) | u32::from(code[1])
            } else {
                cid
            };

            let (w1_y, vx, vy) =
                if font.wmode() == 1 { font.glyph_vertical_metrics(cid) } else { (0.0, 0.0, 0.0) };

            let w = if font.wmode() == 1 { w1_y } else { font.glyph_width_by_cid(cid) };

            let unicode = u.unwrap_or_else(|| {
                if char_code > 31 {
                    std::char::from_u32(0xF0000 + cid)
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "\u{FFFD}".to_string())
                } else {
                    String::new()
                }
            });

            let name = if let Some(ref enc) = font.encoding {
                enc.mappings.get(code).cloned()
            } else {
                None
            };

            let u_char = unicode.chars().next().unwrap_or('\0');
            let resolved_gid = font.resolve_gid(cid, Some(u_char));
            glyphs.push(ferruginous_render::TextGlyph {
                gid: resolved_gid,
                name,
                char_code,
                unicode,
                width: w as f32,
                vx,
                vy,
            });
            i += consumed;
        }
        Ok(glyphs)
    }
}
