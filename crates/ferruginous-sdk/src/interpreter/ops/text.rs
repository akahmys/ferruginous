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
                self.push_name(font);
                self.push_real(*size);
                self.handle_text_state_operator("Tf")
            }
            Command::MoveText(p) => {
                self.push_point(*p);
                self.handle_text_positioning_operator("Td")
            }
            Command::SetTextMatrix(m) => {
                self.push_affine(m);
                self.handle_text_positioning_operator("Tm")
            }
            Command::SetTextRise(f) => {
                self.push_real(*f);
                self.handle_text_state_operator("Ts")
            }
            Command::SetTextLeading(f) => {
                self.push_real(*f);
                self.handle_text_state_operator("TL")
            }
            Command::MoveToNextLine => self.handle_text_positioning_operator("T*"),
            Command::SetCharSpacing(s) => {
                self.push_real(*s);
                self.handle_text_state_operator("Tc")
            }
            Command::SetWordSpacing(s) => {
                self.push_real(*s);
                self.handle_text_state_operator("Tw")
            }
            Command::SetHorizontalScaling(s) => {
                self.push_real(*s);
                self.handle_text_state_operator("Tz")
            }
            Command::SetTextRenderMode(m) => {
                self.push_integer(*m as i64);
                self.handle_text_state_operator("Tr")
            }
            Command::SetWritingMode(w) => {
                self.state.text_state.wmode = *w;
                Ok(())
            }
            Command::Type3SetMetrics { wx, wy, bbox } => {
                if let Some(r) = bbox {
                    self.set_type3_metrics_bbox(*wx, *wy, r.x0, r.y0, r.x1, r.y1)?;
                } else {
                    self.set_type3_metrics(*wx, *wy)?;
                }
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
                let _ = self.resolve_font_resource(&name).map_err(|e| {
                    log::debug!("[SDK] Failed to resolve font {}: {:?}", name.as_str(), e);
                    e
                })?;
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
            "TL" => {
                self.state.text_state.leading = self.pop_f64()?;
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
                m.tm = mat;
                m.tlm = m.tm;
            }
            "T*" => {
                let leading = self.state.text_state.leading;
                let m = self.text_matrices.get_or_insert_with(TextMatrices::default);
                let nl = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -leading);
                m.tlm = m.tlm.concat(&nl);
                m.tm = m.tlm;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles operators that show text (Tj, TJ, ', ").
    pub(crate) fn set_type3_metrics(&mut self, wx: f64, wy: f64) -> PdfResult<()> {
        log::debug!("[SDK] d0 wx={}, wy={}", wx, wy);
        self.type3_advance = Some((wx, wy));
        Ok(())
    }

    pub(crate) fn set_type3_metrics_bbox(
        &mut self,
        wx: f64,
        wy: f64,
        llx: f64,
        lly: f64,
        urx: f64,
        ury: f64,
    ) -> PdfResult<()> {
        log::debug!("[SDK] d1 wx={}, wy={}, bbox=({:?}, {:?}, {:?}, {:?})", wx, wy, llx, lly, urx, ury);
        self.type3_advance = Some((wx, wy));
        Ok(())
    }

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
        let font_name = self.state.text_state.font.as_ref().ok_or_else(|| PdfError::Other("No font".into()))?.clone();
        let res = self.resolve_font_resource(&font_name).map_err(|e| {
            log::debug!("[SDK] show_text failed to resolve font {}: {:?}", font_name.as_str(), e);
            e
        })?;
        let glyphs: Vec<ferruginous_render::TextGlyph> = self.map_text_to_glyphs(text, &res)?;

        let tm = self.text_matrices.as_ref().map(|m| m.tm).unwrap_or_default();
        let rise = self.state.text_state.rise;
        let rise_mat = if res.wmode() == 1 {
            Matrix::new(1.0, 0.0, 0.0, 1.0, rise, 0.0)
        } else {
            Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, rise)
        };
        let render = tm.concat(&rise_mat);

        let font_size = self.state.text_state.font_size;
        let th = self.state.text_state.horizontal_scaling / 100.0;
        let text_state = ferruginous_render::TextState {
            tc: self.state.text_state.char_spacing,
            tw: self.state.text_state.word_spacing,
            th,
            is_vertical: res.wmode() == 1,
        };

        if res.subtype.as_str() == "Type3" {
            // Type 3 is handled by render_type3_glyphs which we call below for advance too
        } else {
            if let Some(_m) = self.text_matrices {
                self.backend.show_text(
                    &glyphs,
                    font_size,
                    render.as_affine(),
                    text_state,
                    self.op_index,
                );
            }
        }

        let is_vertical = res.wmode() == 1;
        let mut total_adv_x = 0.0;
        let mut total_adv_y = 0.0;

        if res.subtype.as_str() == "Type3" {
            let (adv_x, adv_y) = self.render_type3_glyphs(&glyphs)?;
            total_adv_x = adv_x;
            total_adv_y = adv_y;
        } else {
            for glyph in &glyphs {
                let char_width = glyph.width as f64;
                let char_width_pt = char_width / 1000.0 * font_size;
                if is_vertical {
                    let mut adv = char_width_pt * th - self.state.text_state.char_spacing;
                    if glyph.char_code == 0x20 {
                        adv -= self.state.text_state.word_spacing;
                    }
                    total_adv_y += adv;
                } else {
                    let mut adv = (char_width_pt + self.state.text_state.char_spacing) * th;
                    if glyph.char_code == 0x20 {
                        adv += self.state.text_state.word_spacing * th;
                    }
                    total_adv_x += adv;
                }
            }
        }

        let advance_mat = Matrix::new(1.0, 0.0, 0.0, 1.0, total_adv_x, total_adv_y);
        if let Some(m) = self.text_matrices.as_mut() {
            m.tm = m.tm.concat(&advance_mat);
        }
        Ok(())
    }

    pub(crate) fn render_type3_glyphs(
        &mut self,
        glyphs: &[ferruginous_render::TextGlyph],
    ) -> PdfResult<(f64, f64)> {
        let font_name = self.state.text_state.font.as_ref().ok_or_else(|| PdfError::Other("No font".into()))?.clone();
        let res = self.resolve_font_resource(&font_name)?;
        
        let _char_procs = res.char_procs.as_ref().ok_or_else(|| {
            log::warn!("[SDK] Type 3 font {} missing CharProcs", font_name.as_str());
            PdfError::Other("Missing CharProcs".into())
        })?;
        let font_matrix = res.font_matrix.unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);

        let _fm = kurbo::Affine::new(font_matrix.map(|v| v as f64));
        let mut total_adv_x = 0.0;
        let mut total_adv_y = 0.0;

        let font_size = self.state.text_state.font_size;
        let th = self.state.text_state.horizontal_scaling / 100.0;
        log::debug!("[SDK] Rendering Type 3 glyphs: {} glyphs, font_size={}, th={}, FontMatrix={:?}", glyphs.len(), font_size, th, font_matrix);

        for glyph in glyphs {
            let mut glyph_name = glyph.name.clone()
                .unwrap_or_else(|| format!("g{:X}", glyph.char_code));
            
            if glyph_name.starts_with('/') {
                glyph_name = glyph_name[1..].to_string();
            }
            println!("[DIAG] Type 3 Glyph: char_code=0x{:X}, name={:?}", glyph.char_code, glyph_name);

            let stream_h = match res.char_procs.as_ref().and_then(|cp| cp.get(&glyph_name)) {
                Some(h) => h,
                None => {
                    log::debug!("[SDK] Type 3 glyph {} not found in CharProcs", glyph_name);
                    continue;
                }
            };

            let old_state = self.state.clone();
            self.backend.push_state();

            if let Some(m) = self.text_matrices {
                let fm_f32 = res.font_matrix.unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);
                let fm_f64 = [
                    fm_f32[0] as f64, fm_f32[1] as f64, fm_f32[2] as f64,
                    fm_f32[3] as f64, fm_f32[4] as f64, fm_f32[5] as f64,
                ];
                let h_scale = if res.wmode() == 1 { 1.0 } else { th };
                let v_scale = if res.wmode() == 1 { th } else { 1.0 };
                let text_to_pt = kurbo::Affine::scale_non_uniform(font_size * h_scale, -font_size * v_scale);
                let local_to_text = kurbo::Affine::new(fm_f64);
                let translate = kurbo::Affine::translate((total_adv_x, total_adv_y));

                let target_mat = self.state.ctm.as_affine() * m.tm.as_affine() * translate * text_to_pt * local_to_text;
                log::debug!("[TYPE3] target_mat={:?}, ctm={:?}, tm={:?}, translate={:?}, text_to_pt={:?}, local_to_text={:?}", target_mat, self.state.ctm.as_affine(), m.tm.as_affine(), translate, text_to_pt, local_to_text);
                self.state.ctm = Matrix(target_mat.as_coeffs());
                self.backend.set_transform(target_mat);
            }

            self.type3_advance = None;
            self.in_type3_glyph = true;
            if let Err(e) = self.execute(*stream_h) {
                log::error!("[SDK] Failed to execute Type 3 glyph {}: {:?}", glyph_name, e);
            }
            self.backend.pop_state();
            self.state = old_state;
            self.backend.set_transform(self.state.ctm.as_affine());
            self.in_type3_glyph = false;

            let (wx, mut wy) = if let Some(adv) = self.type3_advance {
                adv
            } else {
                (f64::from(glyph.width), 0.0)
            };

            // Fallback for poorly formed Type 3 fonts that don't provide wy in vertical mode
            if res.wmode() == 1 && wy == 0.0 {
                wy = 1000.0;
                log::debug!("[SDK] Fallback wy=1000 for vertical Type 3 glyph");
            }

            let fm_f32 = res.font_matrix.unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);
            let dx_text = fm_f32[0] as f64 * wx + fm_f32[2] as f64 * wy;
            let dy_text = fm_f32[1] as f64 * wx + fm_f32[3] as f64 * wy;
            let mut dx = dx_text * font_size * th;
            let mut dy = dy_text * font_size;

            if res.wmode() == 1 {
                dy -= self.state.text_state.char_spacing;
                if glyph.char_code == 0x20 {
                    dy -= self.state.text_state.word_spacing;
                }
            } else {
                dx += self.state.text_state.char_spacing * th;
                if glyph.char_code == 0x20 {
                    dx += self.state.text_state.word_spacing * th;
                }
            }

            total_adv_x += dx;
            total_adv_y += dy;
        }
        Ok((total_adv_x, total_adv_y))
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

            let unicode_opt = u.or_else(|| {
                if char_code > 31 {
                    std::char::from_u32(0xF0000 + cid).map(|c| c.to_string())
                } else {
                    None
                }
            });
            let unicode = unicode_opt.clone().unwrap_or_else(|| "\u{FFFD}".to_string());

            let name = if let Some(ref enc) = font.encoding {
                enc.mappings.get(code).cloned()
            } else {
                None
            };

            let u_char_hint = unicode_opt.and_then(|s| s.chars().next());
            if let Some(resolved_gid) = font.resolve_gid(cid, u_char_hint, None) {
                glyphs.push(ferruginous_render::TextGlyph {
                    gid: resolved_gid,
                    name,
                    char_code,
                    unicode,
                    width: w as f32,
                    vx,
                    vy,
                });
            }
            i += consumed;
        }
        Ok(glyphs)
    }
}
