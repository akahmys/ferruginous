use std::sync::Arc;
use std::collections::BTreeMap;
use ferruginous_core::{Object, Parser, PdfResult, PdfError, PdfName, lexer::Token};
use ferruginous_render::{RenderBackend, path::PathBuilder};
use ferruginous_core::graphics::{WindingRule, GraphicsState, TextMatrices, TextRenderingMode, Matrix, BlendMode};
use ferruginous_doc::font::{FontResource, cmap::MappingResult};

/// A content stream interpreter that translates PDF operators into [RenderBackend] calls.
pub struct Interpreter<'a> {
    /// The rendering backend used to draw items.
    pub backend: &'a mut dyn RenderBackend,
    /// The resolver used to look up indirect objects.
    pub resolver: &'a dyn ferruginous_core::Resolver,
    /// Stack of resource dictionaries for hierarchical lookup (Form XObjects).
    pub resource_stack: Vec<Arc<BTreeMap<PdfName, Object>>>,
    /// Operand stack for operators.
    stack: Vec<Object>,
    /// Current path being constructed.
    path: PathBuilder,
    /// Graphics state stack (managed by q/Q).
    state_stack: Vec<GraphicsState>,
    /// Current active graphics state.
    state: GraphicsState,
    /// Current text object state (managed by BT/ET).
    text_matrices: Option<TextMatrices>,
    /// Bounding box of the current text object (between BT and ET).
    pub current_text_bbox: Option<ferruginous_core::graphics::Rect>,
    /// Bounding box of all text objects combined on the page.
    pub page_text_bbox: Option<ferruginous_core::graphics::Rect>,
    /// Guard for circular references.
    recursion_depth: usize,
}

impl<'a> Interpreter<'a> {
    /// Creates a new interpreter tied to a specific rendering backend.
    pub fn new(
        backend: &'a mut dyn RenderBackend,
        resolver: &'a dyn ferruginous_core::Resolver,
        initial_resources: Arc<BTreeMap<PdfName, Object>>
    ) -> Self {
        Self {
            backend,
            resolver,
            resource_stack: vec![initial_resources],
            stack: Vec::new(),
            path: PathBuilder::new(),
            state_stack: Vec::new(),
            state: GraphicsState::default(),
            text_matrices: None,
            current_text_bbox: None,
            page_text_bbox: None,
            recursion_depth: 0,
        }
    }

    /// Executes a content stream by parsing and processing its operators.
    pub fn execute(&mut self, data: &[u8]) -> PdfResult<()> {
        let mut parser = Parser::new(bytes::Bytes::copy_from_slice(data))
            .with_resolver(self.resolver);

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

    fn execute_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            // Path Construction (m, l, c, re, h)
            "m" | "l" | "c" | "re" | "h" => self.handle_path_operator(op)?,
            
            // Path Painting (S, f, F, f*)
            "S" | "f" | "F" | "f*" => self.handle_painting_operator(op)?,

            // Graphics State (q, Q, cm, gs)
            "q" | "Q" | "cm" | "gs" => self.handle_state_operator(op)?,

            // Text State (Tc, Tw, Tz, TL, Tf, Tr, Ts)
            "Tc" | "Tw" | "Tz" | "TL" | "Tf" | "Tr" | "Ts" => self.handle_text_state_operator(op)?,

            // Text Objects (BT, ET)
            "BT" | "ET" => self.handle_text_scope_operator(op)?,

            // Text Positioning (Td, TD, Tm, T*)
            "Td" | "TD" | "Tm" | "T*" => self.handle_text_positioning_operator(op)?,

            // Text Showing (Tj, TJ, ', ")
            "Tj" | "TJ" | "'" | "\"" => self.handle_text_showing_operator(op)?,
            
            // XObjects (Do)
            "Do" => self.handle_xobject_operator()?,

            _ => { /* Ignore unknown operators for now */ }
        }
        self.stack.clear();
        Ok(())
    }

    fn handle_path_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "m" => {
                let y = self.pop_f64()?; let x = self.pop_f64()?;
                self.path.move_to(x, y);
            }
            "l" => {
                let y = self.pop_f64()?; let x = self.pop_f64()?;
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
            "h" => self.path.close_path(),
            _ => return Err(PdfError::Other(format!("Invalid path op: {op}"))),
        }
        Ok(())
    }

    fn handle_painting_operator(&mut self, op: &str) -> PdfResult<()> {
        let p = std::mem::replace(&mut self.path, PathBuilder::new()).finish();
        match op {
            "S" => self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style),
            "f" | "F" => self.backend.fill_path(&p, &self.state.fill_color, WindingRule::NonZero),
            "f*" => self.backend.fill_path(&p, &self.state.fill_color, WindingRule::EvenOdd),
            _ => return Err(PdfError::Other(format!("Invalid painting op: {op}"))),
        }
        Ok(())
    }

    #[allow(clippy::many_single_char_names)]
    fn handle_state_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "q" => {
                self.state_stack.push(self.state.clone());
                self.backend.push_state();
            }
            "Q" => {
                self.state = self.state_stack.pop().ok_or_else(|| PdfError::Other("Graphics stack underflow".into()))?;
                self.backend.pop_state();
            }
            "cm" => {
                #[allow(clippy::many_single_char_names)]
                let f = self.pop_f64()?; let e = self.pop_f64()?;
                let d = self.pop_f64()?; let c = self.pop_f64()?;
                let b = self.pop_f64()?; let a = self.pop_f64()?;
                let m = Matrix::new(a, b, c, d, e, f);
                self.state.ctm = self.state.ctm.concat(&m);
                self.backend.transform(m.0);
            }
            "gs" => {
                let name = self.pop_name()?;
                self.handle_gs_operator(&name)?;
            }
            _ => return Err(PdfError::Other(format!("Invalid state op: {op}"))),
        }
        Ok(())
    }

    fn handle_text_state_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Tf" => {
                self.state.text_state.font_size = self.pop_f64()?;
                self.state.text_state.font = Some(self.pop_name()?);
            }
            "Tc" => self.state.text_state.char_spacing = self.pop_f64()?,
            "Tw" => self.state.text_state.word_spacing = self.pop_f64()?,
            "Tz" => self.state.text_state.horizontal_scaling = self.pop_f64()?,
            "TL" => self.state.text_state.leading = self.pop_f64()?,
            "Tr" => self.state.text_state.rendering_mode = TextRenderingMode::from(self.pop_f64()? as i64),
            "Ts" => self.state.text_state.rise = self.pop_f64()?,
            _ => return Err(PdfError::Other(format!("Invalid text state op: {op}"))),
        }
        Ok(())
    }

    fn handle_text_scope_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "BT" => {
                 self.text_matrices = Some(TextMatrices::default());
                 self.current_text_bbox = None;
            }
            "ET" => {
                 if let Some(current) = self.current_text_bbox {
                     self.page_text_bbox = Some(self.page_text_bbox.map_or(current, |p| p.union(&current)));
                 }
                 self.text_matrices = None;
            }
            _ => return Err(PdfError::Other(format!("Invalid text scope op: {op}"))),
        }
        Ok(())
    }

    #[allow(clippy::many_single_char_names)]
    fn handle_text_positioning_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "Td" => {
                let ty = self.pop_f64()?; let tx = self.pop_f64()?;
                let text_matrices = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Td outside of BT/ET scope".into()))?;
                let next_line = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                text_matrices.tlm = text_matrices.tlm.concat(&next_line);
                text_matrices.tm = text_matrices.tlm;
            }
            "TD" => {
                let ty = self.pop_f64()?; let tx = self.pop_f64()?;
                self.state.text_state.leading = -ty;
                let text_matrices = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("TD outside of BT/ET scope".into()))?;
                let next_line = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                text_matrices.tlm = text_matrices.tlm.concat(&next_line);
                text_matrices.tm = text_matrices.tlm;
            }
            "Tm" => {
                #[allow(clippy::many_single_char_names)]
                let f = self.pop_f64()?; let e = self.pop_f64()?;
                let d = self.pop_f64()?; let c = self.pop_f64()?;
                let b = self.pop_f64()?; let a = self.pop_f64()?;
                let text_matrices = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Tm outside of BT/ET scope".into()))?;
                let m = Matrix::new(a, b, c, d, e, f);
                text_matrices.tlm = m;
                text_matrices.tm = m;
            }
            "T*" => {
                let leading = self.state.text_state.leading;
                let text_matrices = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("T* outside of BT/ET scope".into()))?;
                let next_line = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -leading);
                text_matrices.tlm = text_matrices.tlm.concat(&next_line);
                text_matrices.tm = text_matrices.tlm;
            }
            _ => return Err(PdfError::Other(format!("Invalid text positioning op: {op}"))),
        }
        Ok(())
    }

    fn handle_text_showing_operator(&mut self, op: &str) -> PdfResult<()> {
        if self.state.text_state.rendering_mode == TextRenderingMode::Invisible && op != "TJ" {
            self.stack.pop();
            return Ok(());
        }

        match op {
            "Tj" => {
                let s = self.pop_string()?;
                self.show_text(&s)?;
            }
            "TJ" => {
                let arr = self.pop_array()?;
                self.show_text_array(&arr)?;
            }
            "'" => {
                self.handle_text_positioning_operator("T*")?;
                let s = self.pop_string()?;
                self.show_text(&s)?;
            }
            "\"" => {
                let s = self.pop_string()?;
                self.state.text_state.char_spacing = self.pop_f64()?;
                self.state.text_state.word_spacing = self.pop_f64()?;
                self.handle_text_positioning_operator("T*")?;
                self.show_text(&s)?;
            }
            _ => return Err(PdfError::Other(format!("Invalid text showing op: {op}"))),
        }
        Ok(())
    }

    fn show_text(&mut self, text: &[u8]) -> PdfResult<()> {
        let font_name = self.state.text_state.font.clone().ok_or_else(|| PdfError::Other("No font set".into()))?;
        
        // Resolve font resource
        let font_resource = self.resolve_font_resource(&font_name)?;

        let options = ferruginous_render::text::TextLayoutOptions {
            font_size: self.state.text_state.font_size as f32,
            char_spacing: self.state.text_state.char_spacing as f32,
            word_spacing: self.state.text_state.word_spacing as f32,
            horizontal_scaling: self.state.text_state.horizontal_scaling as f32,
        };

        let mut current_pos = 0;
        let mut glyphs = Vec::new();
        
        // Extract font data for rendering
        let font_data = match font_resource.as_ref() {
            FontResource::Simple(f) => f.descriptor.as_ref().and_then(|d| d.font_file.as_ref()),
            FontResource::Composite(f) => {
                f.descendant_fonts.first().and_then(|d| {
                    if let FontResource::CID(cid) = d.as_ref() {
                        cid.descriptor.font_file.as_ref()
                    } else { None }
                })
            }
            FontResource::CID(f) => f.descriptor.font_file.as_ref(),
        };

        let font_data = font_data.ok_or_else(|| PdfError::Other("Missing font stream data".into()))?;

        // Segmentation and mapping
        while current_pos < text.len() {
            let (code, len) = match font_resource.as_ref() {
                FontResource::Composite(f) => {
                    f.encoding.next_code(&text[current_pos..]).unwrap_or((vec![text[current_pos]], 1))
                }
                _ => (vec![text[current_pos]], 1),
            };
            
            let width = font_resource.glyph_width(&code);
            
            let gid = match font_resource.as_ref() {
                FontResource::Composite(f) => {
                    match f.encoding.lookup(&code) {
                        Some(MappingResult::Cid(cid)) => cid,
                        _ => code[0] as u32,
                    }
                }
                FontResource::Simple(_) => {
                    // map byte to unicode via encoding, then could map to GID
                    // For now, let's use the byte as GID (works for many fonts)
                    // or map to unicode if we want to support fallback fonts better
                    code[0] as u32
                }
                FontResource::CID(_) => code[0] as u32,
            };

            glyphs.push((gid, width as f32));
            current_pos += len;
        }

        let bridge = ferruginous_render::text::SkrifaBridge::new();
        let path = bridge.render_glyphs(font_data, &glyphs, &options);

        let text_matrices = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("Tj outside of BT/ET".into()))?;
        let font_metrics = font_resource.get_metrics();
        let font_size = self.state.text_state.font_size;
        let scale = font_size / 1000.0;
        let h_scale = self.state.text_state.horizontal_scaling / 100.0;
        
        // Final transformation including Rise (Ts)
        let render_matrix = text_matrices.tm.concat(&Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, self.state.text_state.rise));
        
        // --- Added: Call backend.show_text for extraction ---
        let mut unicode_buf = String::new();
        let mut current_pos_u = 0;
        while current_pos_u < text.len() {
             let (code, len) = match font_resource.as_ref() {
                FontResource::Composite(f) => {
                    f.encoding.next_code(&text[current_pos_u..]).unwrap_or((vec![text[current_pos_u]], 1))
                }
                _ => (vec![text[current_pos_u]], 1),
            };
            unicode_buf.push_str(&font_resource.to_unicode(&code));
            current_pos_u += len;
        }
        self.backend.show_text(&unicode_buf, font_resource.base_font().as_str(), font_size as f32, render_matrix.0);
        // ----------------------------------------------------

        let mut path = path;
        path.apply_affine(render_matrix.0);
        self.backend.fill_path(&path, &self.state.fill_color, WindingRule::NonZero);

        // Update BBox
        let user_matrix = render_matrix.concat(&self.state.ctm);
        for (_, w) in &glyphs {
            // Glyph box in text space
            let g_box = ferruginous_core::graphics::Rect::new(0.0, font_metrics.descent * scale, (*w as f64) * scale, font_metrics.ascent * scale);
            // Transform to user space using current Tm (including rise) and CTM
            // This is a simplification: we just transform the corners
            let p1 = user_matrix.0 * kurbo::Point::new(g_box.x1, g_box.y1);
            let p2 = user_matrix.0 * kurbo::Point::new(g_box.x2, g_box.y2);
            let u_box = ferruginous_core::graphics::Rect::new(p1.x.min(p2.x), p1.y.min(p2.y), p1.x.max(p2.x), p1.y.max(p2.y));
            
            self.current_text_bbox = Some(self.current_text_bbox.map_or(u_box, |b| b.union(&u_box)));
        }

        // Update Text Matrix (TM)
        let mut total_advance = 0.0;
        for (gid, w) in &glyphs {
            total_advance += (*w as f64).mul_add(scale, self.state.text_state.char_spacing) * h_scale;
            // Word spacing (Tw) applies if space char (GID 32 in simple fonts or specific CIDs)
            if *gid == 32 {
                total_advance += self.state.text_state.word_spacing * h_scale;
            }
        }
        text_matrices.tm = text_matrices.tm.concat(&Matrix::new(1.0, 0.0, 0.0, 1.0, total_advance, 0.0));

        Ok(())
    }

    fn resolve_font_resource(&self, name: &PdfName) -> PdfResult<Arc<FontResource>> {
        let mut font_entry = None;
        for res in self.resource_stack.iter().rev() {
            if let Some(Object::Dictionary(f_dict)) = res.get(&"Font".into()) {
                if let Some(f_ref) = f_dict.get(name) {
                    font_entry = Some(f_ref.clone());
                    break;
                }
            }
        }
        
        let font_obj_ref = font_entry.ok_or_else(|| PdfError::Other(format!("Font {:?} not found", name.0)))?;
        let font_dict_obj = self.resolver.resolve_if_ref(&font_obj_ref)?;
        let font_dict = font_dict_obj.as_dict().ok_or_else(|| PdfError::Other("Invalid font dictionary".into()))?;
        
        Ok(Arc::new(FontResource::load(font_dict, self.resolver)?))
    }

    fn show_text_array(&mut self, arr: &[Object]) -> PdfResult<()> {
        for obj in arr {
            match obj {
                Object::String(s) => self.show_text(s)?,
                Object::Integer(i) => {
                    let offset = -(*i as f64) / 1000.0 * self.state.text_state.font_size;
                    let text_matrices = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("TJ outside of BT/ET".into()))?;
                    text_matrices.tm = text_matrices.tm.concat(&Matrix::new(1.0, 0.0, 0.0, 1.0, offset, 0.0));
                }
                Object::Real(f) => {
                    let offset = -(*f) / 1000.0 * self.state.text_state.font_size;
                    let text_matrices = self.text_matrices.as_mut().ok_or_else(|| PdfError::Other("TJ outside of BT/ET".into()))?;
                    text_matrices.tm = text_matrices.tm.concat(&Matrix::new(1.0, 0.0, 0.0, 1.0, offset, 0.0));
                }
                _ => return Err(PdfError::Other("Invalid object in TJ array".into())),
            }
        }
        Ok(())
    }


    fn pop_string(&mut self) -> PdfResult<bytes::Bytes> {
        match self.stack.pop() {
            Some(Object::String(s)) => Ok(s),
            _ => Err(PdfError::Other("Expected string".into())),
        }
    }

    fn pop_array(&mut self) -> PdfResult<std::sync::Arc<Vec<Object>>> {
        match self.stack.pop() {
            Some(Object::Array(a)) => Ok(a),
            _ => Err(PdfError::Other("Expected array".into())),
        }
    }

    fn pop_name(&mut self) -> PdfResult<PdfName> {
        match self.stack.pop() {
            Some(Object::Name(n)) => Ok(n),
            _ => Err(PdfError::Other("Expected name".into())),
        }
    }

    fn handle_xobject_operator(&mut self) -> PdfResult<()> {
        let name = self.pop_name()?;
        let resolver = self.resolver;
        
        // 1. Resolve /XObject dict from resource stack
        let mut xobjects_dict = None;
        for res in self.resource_stack.iter().rev() {
            if let Some(Object::Dictionary(d)) = res.get(&"XObject".into()) {
                if let Some(xobj_ref) = d.get(&name) {
                    xobjects_dict = Some(xobj_ref.clone());
                    break;
                }
            }
        }
        
        let xobj_ref = xobjects_dict.ok_or_else(|| PdfError::Other(format!("XObject {:?} not found", name.0)))?;
        let xobj = match xobj_ref {
            Object::Reference(r) => resolver.resolve(&r)?,
            _ => xobj_ref,
        };

        // 2. Identify subtype
        let (dict, data) = xobj.as_stream().ok_or_else(|| PdfError::Other("XObject must be a stream".into()))?;
        let subtype = dict.get(&PdfName::from("Subtype")).ok_or_else(|| PdfError::Other("Missing /Subtype in XObject".into()))?;
        let subtype_name = subtype.as_name().ok_or_else(|| PdfError::Other("Invalid /Subtype type".into()))?;

        if subtype_name.as_str() == "Image" {
            self.render_image_xobject(dict, data)?;
        } else if subtype_name.as_str() == "Form" {
            self.render_form_xobject(dict, data)?;
        }

        Ok(())
    }

    fn render_image_xobject(&mut self, dict: &std::collections::BTreeMap<ferruginous_core::PdfName, Object>, data: &[u8]) -> PdfResult<()> {
        let width = dict.get(&PdfName::from("Width")).and_then(|o| o.as_i64()).unwrap_or(0) as u32;
        let height = dict.get(&PdfName::from("Height")).and_then(|o| o.as_i64()).unwrap_or(0) as u32;
        
        // Resolve ColorSpace
        let format = match dict.get(&PdfName::from("ColorSpace")) {
            Some(Object::Name(n)) => match n.as_str() {
                "DeviceGray" | "G" => ferruginous_core::graphics::PixelFormat::Gray8,
                "DeviceCMYK" | "CMYK" => ferruginous_core::graphics::PixelFormat::Cmyk8,
                _ => ferruginous_core::graphics::PixelFormat::Rgb8, // Default to RGB for now
            }
            _ => ferruginous_core::graphics::PixelFormat::Rgb8,
        };

        // Decode data using all specified filters
        let decoded = ferruginous_core::filters::decode_stream_from_dict(dict, data.to_vec())?;

        self.backend.draw_image(&decoded, width, height, format);
        Ok(())
    }

    #[allow(clippy::many_single_char_names)]
    fn render_form_xobject(&mut self, dict: &std::collections::BTreeMap<ferruginous_core::PdfName, Object>, data: &[u8]) -> PdfResult<()> {
        if self.recursion_depth >= 16 {
            return Err(PdfError::Other("Maximum XObject recursion depth reached".into()));
        }

        self.backend.push_state();
        self.state_stack.push(self.state.clone());
        
        // 1. Apply Matrix
        if let Some(matrix_obj) = dict.get(&PdfName::from("Matrix")) {
            if let Some(arr) = matrix_obj.as_array() {
                if arr.len() == 6 {
                    #[allow(clippy::many_single_char_names)]
                    let a = arr[0].as_f64().unwrap_or(1.0);
                    let b = arr[1].as_f64().unwrap_or(0.0);
                    let c = arr[2].as_f64().unwrap_or(0.0);
                    let d = arr[3].as_f64().unwrap_or(1.0);
                    let e = arr[4].as_f64().unwrap_or(0.0);
                    let f = arr[5].as_f64().unwrap_or(0.0);
                    let m = Matrix::new(a, b, c, d, e, f);
                    self.state.ctm = self.state.ctm.concat(&m);
                    self.backend.transform(m.0);
                }
            }
        }

        // 2. Resource shadowing (Push to stack)
        let mut pushed = false;
        if let Some(res_obj) = dict.get(&PdfName::from("Resources")) {
            if let Some(res_dict) = res_obj.as_dict_arc() {
                self.resource_stack.push(res_dict);
                pushed = true;
            }
        }

        // 3. Recursive execute
        self.recursion_depth += 1;
        self.execute(data)?;
        self.recursion_depth -= 1;

        // 4. Restore state
        if pushed {
            self.resource_stack.pop();
        }
        self.state = self.state_stack.pop().unwrap_or_default(); // Should not fail if q/Q balanced
        self.backend.pop_state();

        Ok(())
    }

    fn pop_f64(&mut self) -> PdfResult<f64> {
        match self.stack.pop() {
            Some(Object::Real(f)) => Ok(f),
            Some(Object::Integer(i)) => Ok(i as f64),
            _ => Err(PdfError::Other("Expected number".into())),
        }
    }
    fn handle_gs_operator(&mut self, name: &PdfName) -> PdfResult<()> {
        let resolver = self.resolver;
        
        // 1. Resolve /ExtGState dict from resource stack
        let mut gstate_entry = None;
        for res in self.resource_stack.iter().rev() {
            if let Some(Object::Dictionary(d)) = res.get(&PdfName::from("ExtGState")) {
                if let Some(gs_ref) = d.get(name) {
                    gstate_entry = Some(gs_ref.clone());
                    break;
                }
            }
        }
        
        let gs_ref = gstate_entry.ok_or_else(|| PdfError::Other(format!("ExtGState {:?} not found", name.0)))?;
        let gs_obj = match gs_ref {
            Object::Reference(r) => resolver.resolve(&r)?,
            _ => gs_ref,
        };
        let gs_dict = gs_obj.as_dict().ok_or_else(|| PdfError::Other("Invalid ExtGState dictionary".into()))?;
        
        // 2. Parse /ca (non-stroking alpha)
        if let Some(ca) = gs_dict.get(&PdfName::from("ca")).and_then(|o| o.as_f64()) {
            self.state.fill_alpha = ca;
            self.backend.set_fill_alpha(ca);
        }
        
        // 3. Parse /CA (stroking alpha)
        if let Some(ca_upper) = gs_dict.get(&PdfName::from("CA")).and_then(|o| o.as_f64()) {
            self.state.stroke_alpha = ca_upper;
            self.backend.set_stroke_alpha(ca_upper);
        }
        
        // 4. Parse /BM (Blend Mode)
        if let Some(bm_obj) = gs_dict.get(&PdfName::from("BM")) {
            let bm_name = match bm_obj {
                Object::Name(n) => n.0.clone(),
                Object::Array(a) if !a.is_empty() => {
                    // PDF says BM can be an array of names, pick first available
                    if let Object::Name(n) = &a[0] { n.0.clone() } else { "Normal".into() }
                }
                _ => "Normal".into(),
            };
            let bm_str = String::from_utf8_lossy(&bm_name);
            let mode = <BlendMode as std::str::FromStr>::from_str(&bm_str).unwrap_or(BlendMode::Normal);
            self.state.blend_mode = mode;
            self.backend.set_blend_mode(mode);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferruginous_render::RenderBackend;
    use ferruginous_core::graphics::{Color, WindingRule, StrokeStyle};
    use kurbo::Affine; 

    struct MockBackend;
    impl RenderBackend for MockBackend {
        fn push_state(&mut self) {}
        fn pop_state(&mut self) {}
        fn transform(&mut self, _affine: Affine) {}
        fn fill_path(&mut self, _path: &kurbo::BezPath, _color: &Color, _rule: WindingRule) {}
        fn stroke_path(&mut self, _path: &kurbo::BezPath, _color: &Color, _style: &StrokeStyle) {}
        fn push_clip(&mut self, _path: &kurbo::BezPath, _rule: WindingRule) {}
        fn pop_clip(&mut self) {}
        fn draw_image(&mut self, _data: &[u8], _w: u32, _h: u32, _format: ferruginous_core::graphics::PixelFormat) {}
        fn set_fill_alpha(&mut self, _alpha: f64) {}
        fn set_stroke_alpha(&mut self, _alpha: f64) {}
        fn set_blend_mode(&mut self, _mode: ferruginous_core::graphics::BlendMode) {}
    }

    #[test]
    fn test_recursion_limit() {
        let mut backend = MockBackend;
        struct TestResolver;
        impl ferruginous_core::Resolver for TestResolver {
            fn resolve(&self, _r: &ferruginous_core::Reference) -> PdfResult<Object> { Ok(Object::Null) }
        }
        let resolver = TestResolver;
        let mut interpreter = Interpreter::new(&mut backend, &resolver, Arc::new(BTreeMap::new()));
        
        interpreter.recursion_depth = 15;
        let dict = std::collections::BTreeMap::new();
        let data = b""; 
        
        assert!(interpreter.render_form_xobject(&dict, data).is_ok());
        
        interpreter.recursion_depth = 16;
        assert!(interpreter.render_form_xobject(&dict, data).is_err());
    }

    #[test]
    fn test_state_restoration() {
        let mut backend = MockBackend;
        struct TestResolver;
        impl ferruginous_core::Resolver for TestResolver {
            fn resolve(&self, _r: &ferruginous_core::Reference) -> PdfResult<Object> { Ok(Object::Null) }
        }
        let resolver = TestResolver;
        let mut interpreter = Interpreter::new(&mut backend, &resolver, Arc::new(BTreeMap::new()));
        
        // Initial font size 1.0 (from Default)
        assert!((interpreter.state.text_state.font_size - 1.0).abs() < f64::EPSILON);
        
        // Changing font size line (Tf 12)
        interpreter.execute(b"/F1 12 Tf").unwrap();
        assert!((interpreter.state.text_state.font_size - 12.0).abs() < f64::EPSILON);
        
        // Push state (12.0), change to 1.0, pop back to 12.0
        interpreter.execute(b"q /F1 1 Tf Q").unwrap();
        assert!((interpreter.state.text_state.font_size - 12.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resource_inheritance() {
        let mut backend = MockBackend;
        struct TestResolver;
        impl ferruginous_core::Resolver for TestResolver {
            fn resolve(&self, _r: &ferruginous_core::Reference) -> PdfResult<Object> { Ok(Object::Null) }
        }
        let resolver = TestResolver;
        let mut interpreter = Interpreter::new(&mut backend, &resolver, Arc::new(BTreeMap::new()));
        
        // Push parent resources manually for this specific test
        let mut parent_dict = BTreeMap::new();
        parent_dict.insert("TestKey".into(), Object::Integer(42));
        interpreter.resource_stack.push(Arc::new(parent_dict));
        
        let form_dict = std::collections::BTreeMap::new();
        let data = b"";
        
        interpreter.render_form_xobject(&form_dict, data).unwrap();
        
        // Should be back to the level before the call (which was 2: initial + manual push)
        assert_eq!(interpreter.resource_stack.len(), 2);
        assert!(interpreter.resource_stack[1].contains_key(&"TestKey".into()));
    }

    #[test]
    fn test_ext_gstate_gs_operator() {
        let mut backend = MockBackend;
        struct TestResolver;
        impl ferruginous_core::Resolver for TestResolver {
            fn resolve(&self, _r: &ferruginous_core::Reference) -> PdfResult<Object> { Ok(Object::Null) }
        }
        let resolver = TestResolver;

        let mut gs_dict = std::collections::BTreeMap::new();
        gs_dict.insert(PdfName::from("ca"), Object::Real(0.5));
        gs_dict.insert(PdfName::from("CA"), Object::Real(0.8));
        gs_dict.insert(PdfName::from("BM"), Object::Name(PdfName::from("Multiply")));

        let mut ext_gstate = std::collections::BTreeMap::new();
        ext_gstate.insert(PdfName::from("GS1"), Object::Dictionary(std::sync::Arc::new(gs_dict)));

        let mut resources = std::collections::BTreeMap::new();
        resources.insert(PdfName::from("ExtGState"), Object::Dictionary(std::sync::Arc::new(ext_gstate)));
        
        let mut interpreter = Interpreter::new(&mut backend, &resolver, Arc::new(resources));

        interpreter.handle_gs_operator(&PdfName::from("GS1")).unwrap();

        assert!((interpreter.state.fill_alpha - 0.5).abs() < f64::EPSILON);
        assert!((interpreter.state.stroke_alpha - 0.8).abs() < f64::EPSILON);
        assert_eq!(interpreter.state.blend_mode, BlendMode::Multiply);
    }

    #[test]
    fn test_blend_mode_mapping() {
        use std::str::FromStr;
        assert_eq!(BlendMode::from_str("Multiply").unwrap(), BlendMode::Multiply);
        assert_eq!(BlendMode::from_str("Screen").unwrap(), BlendMode::Screen);
        assert_eq!(BlendMode::from_str("NonExistent").unwrap(), BlendMode::Normal);
    }
}
