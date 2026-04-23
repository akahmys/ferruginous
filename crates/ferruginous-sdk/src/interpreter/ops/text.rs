use crate::interpreter::Interpreter;
use ferruginous_core::{PdfResult, PdfError, Object, Matrix, Handle};
use ferruginous_core::graphics::{TextMatrices, TextRenderingMode};
use ferruginous_core::font::FontResource;

impl Interpreter<'_> {
    pub(crate) fn handle_text_state_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Tf" => {
                let size = self.pop_f64()?;
                let name = self.pop_name()?;
                self.state.text_state.font_size = size;
                self.state.text_state.font = Some(name.clone());
                
                if let Err(e) = self.resolve_font_resource(&name) {
                    eprintln!("WARNING: Failed to resolve font {}: {:?}", name.as_str(), e);
                }
                
                self.backend.set_font(name.as_str());
            }
            "Tr" => {
                let mode_val = self.pop_i64()?;
                let mode = TextRenderingMode::from(mode_val);
                self.state.text_state.rendering_mode = mode;
                self.backend.set_text_render_mode(mode);
            }
            "Tc" => { self.state.text_state.char_spacing = self.pop_f64()?; }
            "Tw" => { self.state.text_state.word_spacing = self.pop_f64()?; }
            "Tz" => { self.state.text_state.horizontal_scaling = self.pop_f64()?; }
            "TL" => { self.state.text_state.leading = self.pop_f64()?; }
            "Ts" => { self.state.text_state.rise = self.pop_f64()?; }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn handle_text_scope_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "BT" => { self.text_matrices = Some(TextMatrices::default()); self.current_text_bbox = None; }
            "ET" => { if let Some(curr) = self.current_text_bbox { self.page_text_bbox = Some(self.page_text_bbox.map_or(curr, |p| p.union(&curr))); } self.text_matrices = None; }
            _ => {}
        }
        Ok(())
    }

    #[allow(clippy::many_single_char_names)]
    pub(crate) fn handle_text_positioning_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Td" => {
                let ty = self.pop_f64()?; let tx = self.pop_f64()?;
                let m = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Not in BT".into()))?;
                let nl = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                m.tlm = m.tlm.concat(&nl); m.tm = m.tlm;
            }
            "TD" => {
                let ty = self.pop_f64()?; let tx = self.pop_f64()?;
                self.state.text_state.leading = -ty; // Setting leading
                let m = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Not in BT".into()))?;
                let nl = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                m.tlm = m.tlm.concat(&nl); m.tm = m.tlm;
            }
            "Tm" => {
                let f = self.pop_f64()?; let e = self.pop_f64()?; let d = self.pop_f64()?; let c = self.pop_f64()?; let b = self.pop_f64()?; let a = self.pop_f64()?;
                let m = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Not in BT".into()))?;
                let mat = Matrix::new(a, b, c, d, e, f);
                m.tlm = mat; m.tm = mat;
            }
            "T*" => {
                let m = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Not in BT".into()))?;
                let nl = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -self.state.text_state.leading);
                m.tlm = m.tlm.concat(&nl); m.tm = m.tlm;
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn handle_text_showing_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Tj" => { let s = self.pop_string()?; self.show_text(&s)?; }
            "TJ" => { let a = self.pop_array()?; self.show_text_array(a)?; }
            "'" => { 
                self.execute_operator("T*")?;
                let s = self.pop_string()?; self.show_text(&s)?; 
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
                self.execute_operator("'")?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn show_text(&mut self, text: &[u8]) -> PdfResult<()> {
        let name = self.state.text_state.font.clone().ok_or_else(|| PdfError::Other("No font".into()))?;
        let res = self.resolve_font_resource(&name)?;
        let (glyphs, uni) = self.map_text_to_glyphs(text, &res)?;

        let m = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Not in BT".into()))?;
        let rise_mat = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, self.state.text_state.rise);
        let render = m.tm.concat(&rise_mat);
        
        // Pass the richer glyph info (gid, advance, vx, vy) to the backend
        self.backend.show_text(&glyphs, &uni, self.state.text_state.font_size, render.as_affine(), self.state.text_state.char_spacing, self.state.text_state.word_spacing, res.wmode() == 1);

        // Update Tm based on text advancement
        let mut total_advance = 0.0;
        for (_, w, _, _, _) in &glyphs {
            total_advance += f64::from(*w) / 1000.0 * self.state.text_state.font_size;
        }
        
        // Add character and word spacing (simplification)
        total_advance += f64::from(u32::try_from(text.len()).unwrap_or(u32::MAX)) * self.state.text_state.char_spacing;

        let advance_mat = if res.wmode() == 1 {
            Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -total_advance)
        } else {
            Matrix::new(1.0, 0.0, 0.0, 1.0, total_advance, 0.0)
        };
        
        let m = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Not in BT".into()))?;
        m.tm = m.tm.concat(&advance_mat);

        Ok(())
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
                    _ if obj.as_f64().is_some() => {
                        let n = obj.as_f64().ok_or_else(|| PdfError::Other("Invalid displacement".into()))?;
                        let displacement = n / 1000.0 * self.state.text_state.font_size;
                        let m = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Not in BT".into()))?;
                        let shift = if wmode == 1 {
                            Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -displacement)
                        } else {
                            Matrix::new(1.0, 0.0, 0.0, 1.0, -displacement, 0.0)
                        };
                        m.tm = m.tm.concat(&shift);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub(crate) fn map_text_to_glyphs(&self, text: &[u8], font: &FontResource) -> PdfResult<(Vec<(u32, f32, f32, f32, u32)>, String)> {
        let mut glyphs = Vec::new();
        let mut uni = String::new();
        
        let mut i = 0;
        while i < text.len() {
            let (consumed, u) = font.decode_next(&text[i..]);
            if consumed == 0 || i + consumed > text.len() { break; }
            
            let code = &text[i..i+consumed];
            let w = font.glyph_width(code);
            let cid = font.to_cid(code);
            
            let char_code: u32 = if consumed == 1 {
                u32::from(code[0])
            } else if consumed == 2 {
                (u32::from(code[0]) << 8) | u32::from(code[1])
            } else {
                cid
            };
            
            let (vx, vy) = if font.wmode() == 1 {
                let (_, vx, vy) = font.glyph_vertical_metrics(cid);
                (vx, vy)
            } else {
                (0.0, 0.0)
            };
            
            if let Some(s) = u {
                // Skip naked null or other unmapped control characters to avoid rendering glitches
                if s == "\0" || s.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
                    i += consumed;
                    continue;
                }
                glyphs.push((cid, w, vx, vy, char_code));
                uni.push_str(&s);
            } else {
                // If no Unicode mapping, we still render the glyph if it's a printable code
                if char_code > 31 || char_code == 0x09 || char_code == 0x0A || char_code == 0x0D {
                    glyphs.push((cid, w, vx, vy, char_code));
                    uni.push('\u{FFFD}');
                }
            }
            
            i += consumed;
        }
        
        Ok((glyphs, uni))
    }
}
