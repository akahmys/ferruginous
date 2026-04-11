//! PDF content stream execution and graphics state processing.
//! (ISO 32000-2:2020 Clause 7.8 and 8.4)

use crate::graphics::{GraphicsStateStack, Color, DrawOp, GlyphInstance, ClippingRule};
use crate::resources::{Resources, RawXObject};
use crate::text::TextState;
use crate::core::{Object, PdfError, PdfResult, ParseErrorVariant};
use crate::text_layer::{TextLayer, TextElement};
use std::collections::BTreeMap;
use std::sync::Arc;
use kurbo::{Affine, BezPath};


/// A single PDF content stream operation consisting of an operator and its operands.
#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    /// The objects serving as operands for the operator.
    pub operands: Vec<Object>,
    /// The operator keyword (e.g., "cm", "Tj").
    pub operator: Vec<u8>,
}

/// A node in the content stream execution tree.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentNode {
    /// A single operation.
    Operation(Operation),
    /// A block of operations (e.g., delimited by q/Q or BT/ET).
    Block(Vec<u8>, Vec<ContentNode>),
}

/// A processor that maintains graphics and text state while executing content nodes.
/// (ISO 32000-2:2020 Clause 7.8.3)
pub struct Processor<'a> {
    /// The graphics state stack (q/Q).
    pub gs_stack: GraphicsStateStack,
    /// The text-specific state (BT/ET).
    pub text_state: TextState,
    /// The resource dictionary for fonts and `XObjects`.
    pub resources: Option<Resources<'a>>,
    /// The currently active font (resolved from TF).
    pub current_font: Option<crate::font::Font>,
    /// The current resource name of the font (for DrawText).
    pub current_font_name: Vec<u8>,
    /// The sequence of drawing operations (DisplayList).
    pub display_list: Vec<DrawOp>,
    /// The collected text layer (if enabled).
    pub text_layer: Option<TextLayer>,
    /// The Optional Content context (Clause 14.11.4).
    pub oc_context: Option<crate::ocg::OCContext>,
    /// Stack of visibility states (BDC/EMC).
    pub visibility_stack: Vec<bool>,
}

impl<'a> Processor<'a> {
    /// Creates a new Processor instance for executing content streams.
    pub fn new(resources: Option<Resources<'a>>, mediabox: Option<[f64; 4]>, oc_context: Option<crate::ocg::OCContext>) -> Self {
        let mut display_list = Vec::new();
        
        // Layer 4: Coordinate Normalization
        if let Some(bbox) = mediabox {
            let height = bbox[3] - bbox[1];
            // Y-flip: [1 0 0 -1 0 height]
            let m = Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, height]);
            display_list.push(DrawOp::SetTransform(m));
        }

        Self {
            gs_stack: GraphicsStateStack::new(),
            text_state: TextState::default(),
            resources,
            current_font: None,
            current_font_name: Vec::new(),
            display_list,
            text_layer: None,
            oc_context,
            visibility_stack: vec![true],
        }
    }

    fn current_visibility(&self) -> bool {
        *self.visibility_stack.last().unwrap_or(&true)
    }

    /// Enables text extraction by initializing a `TextLayer`.
    pub fn enable_text_extraction(&mut self) {
        self.text_layer = Some(TextLayer::new());
    }

    /// Processes a sequence of content nodes non-recursively.
    /// This is the primary entry point for executing a display list.
    /// (ISO 32000-2:2020 Clause 7.8.2)
    pub fn process_nodes(&mut self, nodes: &[ContentNode]) -> PdfResult<()> {
        let mut owned_nodes = nodes.to_vec();
        owned_nodes.reverse();

        // Stack of (reversed_node_list, end_op)
        let mut stack: Vec<(Vec<ContentNode>, Option<Vec<u8>>)> = vec![(owned_nodes, None)];
        let mut loop_count = 0;
        const MAX_ITER: usize = 100_000;

        while let Some((current_list, _)) = stack.last_mut() {
            loop_count += 1;
            if loop_count > MAX_ITER { 
                return Err(PdfError::ContentError("Content stream execution limit exceeded".into())); 
            }

            if let Some(node) = current_list.pop() {
                self.process_single_node(node, &mut stack)?;
            } else {
                self.finalize_node_list(&mut stack)?;
            }
        }
        Ok(())
    }

    fn process_single_node(
        &mut self, 
        node: ContentNode, 
        stack: &mut Vec<(Vec<ContentNode>, Option<Vec<u8>>)>
    ) -> PdfResult<()> {
        match node {
            ContentNode::Operation(op) => {
                if let Some(mut form_nodes) = self.execute_operation(&op)? {
                    self.gs_stack.push()?;
                    form_nodes.reverse();
                    stack.push((form_nodes, Some(b"q".to_vec())));
                }
            }
            ContentNode::Block(start_op, mut children) => {
                self.setup_block(&start_op)?;
                let end_op_inner = if start_op == b"q" || start_op == b"BT" {
                    Some(start_op.clone())
                } else {
                    None
                };
                children.reverse();
                stack.push((children, end_op_inner));
            }
        }
        Ok(())
    }

    fn setup_block(&mut self, start_op: &[u8]) -> PdfResult<()> {
        if start_op == b"q" {
            self.gs_stack.push()?;
            self.display_list.push(DrawOp::PushState);
        } else if start_op == b"BT" {
            self.text_state.begin_text();
        }
        Ok(())
    }

    fn finalize_node_list(&mut self, stack: &mut Vec<(Vec<ContentNode>, Option<Vec<u8>>)>) -> PdfResult<()> {
        let (_, finished_op) = stack.pop().ok_or_else(|| PdfError::ContentError("Stack underflow".into()))?;
        if let Some(op) = finished_op {
            if op == b"q" {
                self.gs_stack.pop()?;
                self.display_list.push(DrawOp::PopState);
            } else if op == b"BT" {
                // ET
            }
        }
        Ok(())
    }

    /// Executes a single PDF graphics or text operation.
    pub fn execute_operation(&mut self, op: &Operation) -> PdfResult<Option<Vec<ContentNode>>> {
        match op.operator.as_slice() {
            b"cm" => self.handle_cm(op).map(|()| None),
            b"Td" => self.handle_td(op).map(|()| None),
            b"Tm" => self.handle_tm(op).map(|()| None),
            b"Tf" => self.handle_tf(op).map(|()| None),
            b"Tc" => self.handle_tc(op).map(|()| None),
            b"Tw" => self.handle_tw(op).map(|()| None),
            b"Th" => self.handle_th(op).map(|()| None),
            b"Tl" => self.handle_tl(op).map(|()| None),
            b"Ts" => self.handle_ts(op).map(|()| None),
            b"Tr" => self.handle_tr(op).map(|()| None),
            b"Tj" => self.handle_tj(op).map(|()| None),
            b"TJ" => self.handle_tj_array(op).map(|()| None),
            b"Do" => self.handle_do(op),
            b"m" => self.handle_m(op).map(|()| None),
            b"l" => self.handle_l(op).map(|()| None),
            b"c" => self.handle_c(op).map(|()| None),
            b"h" => self.handle_h(op).map(|()| None),
            b"S" => self.handle_s(op).map(|()| None),
            b"s" => self.handle_close_stroke(op).map(|()| None),
            b"f" | b"F" => self.handle_f(op).map(|()| None),
            b"f*" => self.handle_f_star(op).map(|()| None),
            b"B" => self.handle_b_fill_stroke(op).map(|()| None),
            b"B*" => self.handle_b_star_fill_stroke(op).map(|()| None),
            b"b" => self.handle_close_fill_stroke(op).map(|()| None),
            b"b*" => self.handle_close_star_fill_stroke(op).map(|()| None),
            b"n" => self.handle_n_no_op(op).map(|()| None),
            b"W" => self.handle_w_clip(op).map(|()| None),
            b"W*" => self.handle_w_star_clip(op).map(|()| None),
            b"g" => self.handle_g(op).map(|()| None),
            b"G" => self.handle_g_upper(op).map(|()| None),
            b"rg" => self.handle_rg(op).map(|()| None),
            b"RG" => self.handle_rg_upper(op).map(|()| None),
            b"k" => self.handle_k(op).map(|()| None),
            b"K" => self.handle_k_upper(op).map(|()| None),
            b"cs" => self.handle_cs(op).map(|()| None),
            b"CS" => self.handle_cs_upper(op).map(|()| None),
            b"sc" => self.handle_sc(op).map(|()| None),
            b"SC" => self.handle_sc_upper(op).map(|()| None),
            b"scn" => self.handle_scn(op).map(|()| None),
            b"SCN" => self.handle_scn_upper(op).map(|()| None),
            b"gs" => self.handle_gs(op).map(|()| None),
            b"sh" => self.handle_sh(op).map(|()| None),
            b"BDC" => self.handle_bdc(op).map(|()| None),
            b"EMC" => self.handle_emc(op).map(|()| None),
            b"BMC" => self.handle_bmc(op).map(|()| None),
            _ => Ok(None), 
        }
    }

    fn handle_cm(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f64_operands(&op.operands, 6);
        if v.len() == 6 {
            let m = Affine::new([v[0], v[1], v[2], v[3], v[4], v[5]]);
            self.gs_stack.current_mut()?.ctm = self.gs_stack.current()?.ctm * m;
            // Emit transform command
            self.display_list.push(DrawOp::SetTransform(m));
        }
        Ok(())
    }

    fn handle_td(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f64_operands(&op.operands, 2);
        if v.len() == 2 {
            self.text_state.move_text(v[0], v[1]);
        }
        Ok(())
    }

    fn handle_tm(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f64_operands(&op.operands, 6);
        if v.len() == 6 {
            self.text_state.set_matrix(v[0], v[1], v[2], v[3], v[4], v[5]);
        }
        Ok(())
    }

    fn handle_tf(&mut self, op: &Operation) -> PdfResult<()> {
        if op.operands.len() == 2 {
            if let Some(Object::Name(name)) = op.operands.first() {
                self.text_state.font = Some(Object::Name(name.clone()));
                
                // Resolve font if resources are available
                if let Some(ref res) = self.resources {
                    self.current_font_name = name.to_vec();
                    if let Some(font_dict) = res.get_font(name) {
                        if let Ok(font) = crate::font::Font::from_dict(&font_dict, res.resolver) {
                            self.text_state.wmode = if font.is_vertical() { 1 } else { 0 };
                            self.current_font = Some(font);
                        }
                    }
                }
            }

            if let Some(&Object::Real(s)) = op.operands.get(1) {
                self.text_state.font_size = s;
            } else if let Some(&Object::Integer(i)) = op.operands.get(1) {
                self.text_state.font_size = i as f64;
            }
        }
        Ok(())
    }

    fn handle_tc(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(v) = self.extract_f64_operands(&op.operands, 1).first() {
            self.text_state.char_spacing = *v;
        }
        Ok(())
    }

    fn handle_tw(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(v) = self.extract_f64_operands(&op.operands, 1).first() {
            self.text_state.word_spacing = *v;
        }
        Ok(())
    }

    fn handle_th(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(v) = self.extract_f64_operands(&op.operands, 1).first() {
            self.text_state.horizontal_scaling = *v;
        }
        Ok(())
    }

    fn handle_tl(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(v) = self.extract_f64_operands(&op.operands, 1).first() {
            self.text_state.leading = *v;
        }
        Ok(())
    }

    fn handle_ts(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(v) = self.extract_f64_operands(&op.operands, 1).first() {
            self.text_state.text_rise = *v;
        }
        Ok(())
    }

    fn handle_tr(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(Object::Integer(i)) = op.operands.first() {
            self.text_state.rendering_mode = *i as i32;
        }
        Ok(())
    }

    fn handle_tj(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(Object::String(bytes)) = op.operands.first() {
            let mut glyphs = Vec::new();
            let mut i = 0;
            let mut loop_count = 0;
            while i < bytes.len() {
                loop_count += 1;
                debug_assert!(loop_count <= 10_000, "handle_tj: excessive glyphs");
                if loop_count > 10_000 { break; }

                let len = self.current_font.as_ref()
                    .and_then(|f| f.encoding_cmap.as_ref())
                    .map_or(1, |cmap| cmap.code_length(&bytes[i..]));
                
                let char_code = bytes[i..i+len].to_vec();
                let width = self.current_font.as_ref()
                    .map_or(0.0, |f| f.char_width(&char_code));
                
                let is_space = char_code == [32];
                let (_, matrix_before) = self.text_state.advance_glyph(is_space, width, 0.0);
                
                // Calculate BBox and Path in page space
                let (bbox, path) = self.calculate_glyph_data(&char_code, matrix_before);
                
                glyphs.push(GlyphInstance { char_code, x_advance: width, bbox, path });
                i += len;
            }

            if self.text_layer.is_some() && self.current_visibility() {
                self.record_text_element(&glyphs);
            }

            if self.current_visibility() {
                let gs = self.gs_stack.current()?;
                let resolved_color = gs.fill_color.to_rgb(&gs.fill_color_space);
                self.display_list.push(DrawOp::DrawText {
                    glyphs,
                    font_id: self.current_font_name.clone(),
                    size: self.text_state.font_size,
                    color: Color::RGB(resolved_color[0], resolved_color[1], resolved_color[2]),
                    blend_mode: gs.blend_mode,
                    alpha: gs.fill_alpha as f32,
                });
            }
        }
        Ok(())
    }

    fn handle_tj_array(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(Object::Array(arr)) = op.operands.first() {
            let mut glyphs = Vec::new();
            for item in arr.iter() {
                match item {
                    Object::String(bytes) => self.process_string_glyphs(bytes, &mut glyphs),
                    Object::Integer(i) => self.apply_text_adjustment(*i as f64, &mut glyphs),
                    Object::Real(r) => self.apply_text_adjustment(*r, &mut glyphs),
                    _ => {}
                }
            }

            if self.text_layer.is_some() && self.current_visibility() {
                self.record_text_element(&glyphs);
            }

            if self.current_visibility() {
                let gs = self.gs_stack.current()?;
                let resolved_color = gs.fill_color.to_rgb(&gs.fill_color_space);
                self.display_list.push(DrawOp::DrawText {
                    glyphs,
                    font_id: self.current_font_name.clone(),
                    size: self.text_state.font_size,
                    color: Color::RGB(resolved_color[0], resolved_color[1], resolved_color[2]),
                    blend_mode: gs.blend_mode,
                    alpha: gs.fill_alpha as f32,
                });
            }
        }
        Ok(())
    }

    fn record_text_element(&mut self, glyphs: &[GlyphInstance]) {
        let (font_name, font_size, matrix, color) = (
            self.current_font_name.clone(),
            self.text_state.font_size,
            self.text_state.matrix,
            self.gs_stack.current().map(|gs| gs.fill_color.clone()).unwrap_or(Color::Gray(0.0)),
        );

        let mut text = String::new();
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        if let Some(ref font) = self.current_font {
            for g in glyphs {
                text.push_str(&font.to_unicode_string(&g.char_code));
                min_x = min_x.min(g.bbox.x0);
                min_y = min_y.min(g.bbox.y0);
                max_x = max_x.max(g.bbox.x1);
                max_y = max_y.max(g.bbox.y1);
            }
        }

        if !text.is_empty() {
            if let Some(ref mut layer) = self.text_layer {
                layer.add_element(TextElement {
                    text,
                    bbox: kurbo::Rect::new(min_x, min_y, max_x, max_y),
                    font_name,
                    font_size,
                    matrix,
                    color,
                });
            }
        }
    }

    fn process_string_glyphs(&mut self, bytes: &[u8], glyphs: &mut Vec<GlyphInstance>) {
        let mut i = 0;
        let mut loop_count = 0;
        while i < bytes.len() {
            loop_count += 1;
            if loop_count > 10_000 { break; }

            let len = self.current_font.as_ref()
                .and_then(|f| f.encoding_cmap.as_ref())
                .map_or(1, |cmap| cmap.code_length(&bytes[i..]));
            
            let char_code = bytes[i..i+len].to_vec();
            let width = self.current_font.as_ref()
                .map_or(0.0, |f| f.char_width(&char_code));
            
            let is_space = char_code == [32];
            let (_, matrix_before) = self.text_state.advance_glyph(is_space, width, 0.0);
            
            let (bbox, path) = self.calculate_glyph_data(&char_code, matrix_before);
            
            glyphs.push(GlyphInstance { char_code, x_advance: width, bbox, path });
            i += len;
        }
    }

    fn calculate_glyph_data(&self, char_code: &[u8], tm: Affine) -> (kurbo::Rect, Option<Arc<BezPath>>) {
        let font = match self.current_font {
            Some(ref f) => f,
            None => return (kurbo::Rect::ZERO, None),
        };

        let glyph_bbox = font.glyph_bbox(char_code);
        let glyph_path = font.get_glyph_path(char_code).ok().flatten();
        
        let fs = self.text_state.font_size;
        let th = self.text_state.horizontal_scaling / 100.0;
        let rise = self.text_state.text_rise;
        
        let text_extra = Affine::new([fs * th, 0.0, 0.0, fs, 0.0, rise]);
        let glyph_to_text = tm * text_extra * Affine::scale(0.001);
        
        let ctm = self.gs_stack.current().map(|gs| gs.ctm).unwrap_or(Affine::IDENTITY);
        let total_matrix = ctm * glyph_to_text;

        let res_bbox = total_matrix.transform_rect_bbox(glyph_bbox);
        let res_path = glyph_path.map(|mut p| {
            p.apply_affine(total_matrix);
            Arc::new(p)
        });

        (res_bbox, res_path)
    }

    fn apply_text_adjustment(&mut self, d: f64, glyphs: &mut Vec<GlyphInstance>) {
        let (_, _) = self.text_state.advance_glyph(false, 0.0, d);
        if let Some(last) = glyphs.last_mut() {
            last.x_advance -= d / 1000.0;
            // Note: BBox is calculated based on TM *before* advancement.
            // Adjustments like Tj typically move the cursor for the *next* glyph.
        }
    }

    fn handle_m(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f64_operands(&op.operands, 2);
        if v.len() == 2 {
            std::sync::Arc::make_mut(&mut self.gs_stack.current_mut()?.current_path).move_to((v[0], v[1]));
        }
        Ok(())
    }

    fn handle_l(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f64_operands(&op.operands, 2);
        if v.len() == 2 {
            std::sync::Arc::make_mut(&mut self.gs_stack.current_mut()?.current_path).line_to((v[0], v[1]));
        }
        Ok(())
    }

    fn handle_c(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f64_operands(&op.operands, 6);
        if v.len() == 6 {
            std::sync::Arc::make_mut(&mut self.gs_stack.current_mut()?.current_path).curve_to((v[0], v[1]), (v[2], v[3]), (v[4], v[5]));
        }
        Ok(())
    }

    fn handle_do(&mut self, op: &Operation) -> PdfResult<Option<Vec<ContentNode>>> {
        if !self.current_visibility() { return Ok(None); }
        if let Some(Object::Name(name)) = op.operands.first() {
            if let Some(ref res) = self.resources {
                if let Some(xobj) = res.get_xobject(name) {
                    let subtype = match xobj.dictionary.get(b"Subtype".as_ref()) {
                        Some(Object::Name(n)) => n,
                        _ => return Ok(None),
                    };

                    if subtype.as_slice() == b"Form" {
                        return self.handle_form_xobject(&xobj);
                    } else if subtype.as_slice() == b"Image" {
                        self.handle_image_xobject(&xobj)?;
                    }
                }
            }
        }
        Ok(None)
    }

    /// Handles Form XObjects by parsing their content stream and prefixing a transformation matrix.
    /// (ISO 32000-2:2020 Clause 8.10)
    fn handle_form_xobject(&self, xobj: &RawXObject) -> PdfResult<Option<Vec<ContentNode>>> {
        let mut nodes = parse_content_stream(&xobj.data)?;
        
        if let Some(Object::Array(m_arr)) = xobj.dictionary.get(b"Matrix".as_ref()) {
            let v = self.extract_f64_operands(m_arr, 6);
            if v.len() == 6 {
                let cm_op = Operation {
                    operator: b"cm".to_vec(),
                    operands: v.iter().map(|&f| Object::Real(f)).collect(),
                };
                nodes.insert(0, ContentNode::Operation(cm_op));
            }
        }
        
        Ok(Some(nodes))
    }

    fn handle_image_xobject(&mut self, xobj: &RawXObject) -> PdfResult<()> {
        let width = xobj.dictionary.get(b"Width".as_ref()).and_then(|o| if let Object::Integer(i) = o { Some(*i as u32) } else { None }).unwrap_or(0);
        let height = xobj.dictionary.get(b"Height".as_ref()).and_then(|o| if let Object::Integer(i) = o { Some(*i as u32) } else { None }).unwrap_or(0);
        
        let cs = xobj.dictionary.get(b"ColorSpace".as_ref());
        let components = match cs {
            Some(Object::Name(n)) if n.as_slice() == b"DeviceRGB" => 3,
            Some(Object::Name(n)) if n.as_slice() == b"DeviceGray" => 1,
            _ => 3,
        };

        let decoded_data = crate::filter::decode_stream(&xobj.dictionary, &xobj.data)?;
        let rect = kurbo::Rect::new(0.0, 0.0, 1.0, 1.0);

        let gs = self.gs_stack.current()?;
        self.display_list.push(DrawOp::DrawImage {
            data: Arc::new(decoded_data),
            width,
            height,
            components,
            rect,
            blend_mode: gs.blend_mode,
            alpha: gs.fill_alpha as f32,
        });
        Ok(())
    }

    fn handle_h(&mut self, _op: &Operation) -> PdfResult<()> {
        Arc::make_mut(&mut self.gs_stack.current_mut()?.current_path).close_path();
        Ok(())
    }

    fn handle_s(&mut self, _op: &Operation) -> PdfResult<()> {
        if !self.current_visibility() {
            self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
            return Ok(());
        }
        self.apply_pending_clipping()?;
        self.handle_s_stroke(_op)?;
        self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
        Ok(())
    }

    fn handle_s_stroke(&mut self, _op: &Operation) -> PdfResult<()> {
        if !self.current_visibility() { return Ok(()); }
        let gs = self.gs_stack.current()?;
        let resolved_color = gs.stroke_color.to_rgb(&gs.stroke_color_space);
        self.display_list.push(DrawOp::StrokePath {
            path: std::sync::Arc::clone(&gs.current_path),
            color: Color::RGB(resolved_color[0], resolved_color[1], resolved_color[2]),
            width: gs.line_width,
            blend_mode: gs.blend_mode,
            alpha: gs.stroke_alpha as f32,
        });
        Ok(())
    }

    fn handle_close_stroke(&mut self, _op: &Operation) -> PdfResult<()> {
        Arc::make_mut(&mut self.gs_stack.current_mut()?.current_path).close_path();
        self.handle_s(_op)
    }

    fn handle_f(&mut self, _op: &Operation) -> PdfResult<()> {
        if !self.current_visibility() {
            self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
            return Ok(());
        }
        self.apply_pending_clipping()?;
        self.handle_f_fill(_op)?;
        self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
        Ok(())
    }

    fn handle_f_fill(&mut self, _op: &Operation) -> PdfResult<()> {
        if !self.current_visibility() { return Ok(()); }
        let gs = self.gs_stack.current()?;
        let resolved_color = gs.fill_color.to_rgb(&gs.fill_color_space);
        self.display_list.push(DrawOp::FillPath {
            path: std::sync::Arc::clone(&gs.current_path),
            color: Color::RGB(resolved_color[0], resolved_color[1], resolved_color[2]),
            rule: ClippingRule::NonZeroWinding,
            blend_mode: gs.blend_mode,
            alpha: gs.fill_alpha as f32,
        });
        Ok(())
    }

    fn handle_f_star(&mut self, _op: &Operation) -> PdfResult<()> {
        if !self.current_visibility() {
            self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
            return Ok(());
        }
        self.apply_pending_clipping()?;
        let gs = self.gs_stack.current()?;
        self.display_list.push(DrawOp::FillPath {
            path: Arc::clone(&gs.current_path),
            color: gs.fill_color.clone(),
            rule: ClippingRule::EvenOdd,
            blend_mode: gs.blend_mode,
            alpha: gs.fill_alpha as f32,
        });
        self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
        Ok(())
    }

    fn handle_b_fill_stroke(&mut self, _op: &Operation) -> PdfResult<()> {
        if !self.current_visibility() {
            self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
            return Ok(());
        }
        self.apply_pending_clipping()?;
        self.handle_f_fill(_op)?;
        self.handle_s_stroke(_op)?;
        self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
        Ok(())
    }

    fn handle_b_star_fill_stroke(&mut self, _op: &Operation) -> PdfResult<()> {
        if !self.current_visibility() {
            self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
            return Ok(());
        }
        self.apply_pending_clipping()?;
        let gs = self.gs_stack.current()?;
        self.display_list.push(DrawOp::FillPath {
            path: Arc::clone(&gs.current_path),
            color: gs.fill_color.clone(),
            rule: ClippingRule::EvenOdd,
            blend_mode: gs.blend_mode,
            alpha: gs.fill_alpha as f32,
        });
        self.display_list.push(DrawOp::StrokePath {
            path: Arc::clone(&gs.current_path),
            color: gs.stroke_color.clone(),
            width: gs.line_width,
            blend_mode: gs.blend_mode,
            alpha: gs.stroke_alpha as f32,
        });
        self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
        Ok(())
    }

    fn handle_close_fill_stroke(&mut self, _op: &Operation) -> PdfResult<()> {
        Arc::make_mut(&mut self.gs_stack.current_mut()?.current_path).close_path();
        self.handle_b_fill_stroke(_op)
    }

    fn handle_close_star_fill_stroke(&mut self, _op: &Operation) -> PdfResult<()> {
        Arc::make_mut(&mut self.gs_stack.current_mut()?.current_path).close_path();
        self.handle_b_star_fill_stroke(_op)
    }

    fn handle_n_no_op(&mut self, _op: &Operation) -> PdfResult<()> {
        self.apply_pending_clipping()?;
        self.gs_stack.current_mut()?.current_path = Arc::new(BezPath::new());
        Ok(())
    }

    fn handle_w_clip(&mut self, _op: &Operation) -> PdfResult<()> {
        let gs = self.gs_stack.current_mut()?;
        let path = Arc::clone(&gs.current_path);
        
        // Finalize the path for clipping (ISO 32000-2:2020 Clause 8.5.4)
        Arc::make_mut(&mut gs.clipping_path).extend(path.iter());
        gs.pending_clipping_rule = Some(ClippingRule::NonZeroWinding);
        Ok(())
    }

    fn handle_w_star_clip(&mut self, _op: &Operation) -> PdfResult<()> {
        self.gs_stack.current_mut()?.pending_clipping_rule = Some(ClippingRule::EvenOdd);
        Ok(())
    }

    fn handle_g(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(v) = self.extract_f32_operands(&op.operands, 1).first() {
            self.gs_stack.current_mut()?.fill_color = Color::Gray(*v);
            self.gs_stack.current_mut()?.fill_color_space = crate::colorspace::ColorSpace::DeviceGray;
        }
        Ok(())
    }

    fn handle_g_upper(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(v) = self.extract_f32_operands(&op.operands, 1).first() {
            self.gs_stack.current_mut()?.stroke_color = Color::Gray(*v);
            self.gs_stack.current_mut()?.stroke_color_space = crate::colorspace::ColorSpace::DeviceGray;
        }
        Ok(())
    }

    fn handle_rg(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f32_operands(&op.operands, 3);
        if v.len() == 3 {
            self.gs_stack.current_mut()?.fill_color = Color::RGB(v[0], v[1], v[2]);
            self.gs_stack.current_mut()?.fill_color_space = crate::colorspace::ColorSpace::DeviceRGB;
        }
        Ok(())
    }

    fn handle_rg_upper(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f32_operands(&op.operands, 3);
        if v.len() == 3 {
            self.gs_stack.current_mut()?.stroke_color = Color::RGB(v[0], v[1], v[2]);
            self.gs_stack.current_mut()?.stroke_color_space = crate::colorspace::ColorSpace::DeviceRGB;
        }
        Ok(())
    }

    fn handle_k(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f32_operands(&op.operands, 4);
        if v.len() == 4 {
            self.gs_stack.current_mut()?.fill_color = Color::CMYK(v[0], v[1], v[2], v[3]);
            self.gs_stack.current_mut()?.fill_color_space = crate::colorspace::ColorSpace::DeviceCMYK;
        }
        Ok(())
    }

    fn handle_k_upper(&mut self, op: &Operation) -> PdfResult<()> {
        let v = self.extract_f32_operands(&op.operands, 4);
        if v.len() == 4 {
            self.gs_stack.current_mut()?.stroke_color = Color::CMYK(v[0], v[1], v[2], v[3]);
            self.gs_stack.current_mut()?.stroke_color_space = crate::colorspace::ColorSpace::DeviceCMYK;
        }
        Ok(())
    }

    fn handle_cs(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(Object::Name(name)) = op.operands.first() {
            if let Some(ref res) = self.resources {
                let cs = if let Some(cs_obj) = res.get_color_space(name) {
                    crate::colorspace::ColorSpace::from_object(&cs_obj, res.resolver)?
                } else {
                    crate::colorspace::ColorSpace::from_object(&Object::Name(name.clone()), res.resolver)?
                };
                self.gs_stack.current_mut()?.stroke_color_space = cs;
            }
        }
        Ok(())
    }

    fn handle_cs_upper(&mut self, op: &Operation) -> PdfResult<()> {
        if let Some(Object::Name(name)) = op.operands.first() {
            if let Some(ref res) = self.resources {
                let cs = if let Some(cs_obj) = res.get_color_space(name) {
                    crate::colorspace::ColorSpace::from_object(&cs_obj, res.resolver)?
                } else {
                    crate::colorspace::ColorSpace::from_object(&Object::Name(name.clone()), res.resolver)?
                };
                self.gs_stack.current_mut()?.fill_color_space = cs;
            }
        }
        Ok(())
    }

    fn handle_sc(&mut self, op: &Operation) -> PdfResult<()> {
        let n = self.gs_stack.current()?.stroke_color_space.components();
        let v = self.extract_f32_operands(&op.operands, n as usize);
        if v.len() == n as usize {
            self.gs_stack.current_mut()?.stroke_color = match n {
                1 => Color::Gray(v[0]),
                3 => Color::RGB(v[0], v[1], v[2]),
                4 => Color::CMYK(v[0], v[1], v[2], v[3]),
                _ => Color::ICC(v),
            };
        }
        Ok(())
    }

    fn handle_sc_upper(&mut self, op: &Operation) -> PdfResult<()> {
        let n = self.gs_stack.current()?.fill_color_space.components();
        let v = self.extract_f32_operands(&op.operands, n as usize);
        if v.len() == n as usize {
            self.gs_stack.current_mut()?.fill_color = match n {
                1 => Color::Gray(v[0]),
                3 => Color::RGB(v[0], v[1], v[2]),
                4 => Color::CMYK(v[0], v[1], v[2], v[3]),
                _ => Color::ICC(v),
            };
        }
        Ok(())
    }

    fn handle_scn(&mut self, op: &Operation) -> PdfResult<()> {
        self.handle_sc(op) // For now, similar to sc unless pattern
    }

    fn handle_scn_upper(&mut self, op: &Operation) -> PdfResult<()> {
        self.handle_sc_upper(op)
    }

    fn apply_pending_clipping(&mut self) -> PdfResult<()> {
        let rule = {
            let gs = self.gs_stack.current()?;
            gs.pending_clipping_rule
        };
        
        if let Some(r) = rule {
            let gs = self.gs_stack.current_mut()?;
            let path = gs.current_path.clone(); // Arc clone
            // In PDF, clipping paths accumulate (intersection)
            // For now, we extend the clipping path in the graphics state
            Arc::make_mut(&mut gs.clipping_path).extend(path.iter());
            gs.pending_clipping_rule = None;
            // Emit clip command
            self.display_list.push(DrawOp::Clip(path, r));
        }
        Ok(())
    }

    fn extract_f64_operands(&self, operands: &[Object], count: usize) -> Vec<f64> {
        operands.iter().take(count).filter_map(|o| match o {
            Object::Integer(i) => Some(*i as f64),
            Object::Real(f) => Some(*f),
            _ => None,
        }).collect()
    }

    fn extract_f32_operands(&self, operands: &[Object], count: usize) -> Vec<f32> {
        operands.iter().take(count).filter_map(|o| match o {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(f) => Some(*f as f32),
            _ => None,
        }).collect()
    }

    fn handle_gs(&mut self, op: &Operation) -> PdfResult<()> {
        debug_assert!(!op.operands.is_empty(), "handle_gs: operand missing");
        if let Some(Object::Name(name)) = op.operands.first() {
            if let Some(ref res) = self.resources {
                if let Some(dict) = res.get_sub_dict(b"ExtGState") {
                    if let Some(gs_obj) = dict.get(name.as_slice()) {
                        let actual_gs = if let Object::Reference(r) = gs_obj {
                            res.resolver.resolve(r)?
                        } else { gs_obj.clone() };

                        if let Object::Dictionary(gs_dict) = actual_gs {
                            self.apply_ext_gstate(&gs_dict)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_sh(&mut self, op: &Operation) -> PdfResult<()> {
        debug_assert!(!op.operands.is_empty(), "handle_sh: operand missing");
        if let Some(Object::Name(name)) = op.operands.first() {
            if let Some(ref res) = self.resources {
                if let Some(sh_dict) = res.get_shading(name) {
                    let gs = self.gs_stack.current()?;
                    let shading = crate::graphics::Shading::from_dict(&sh_dict, res.resolver)?;
                     self.display_list.push(DrawOp::DrawShading {
                         shading: Arc::new(shading),
                         blend_mode: gs.blend_mode,
                         alpha: gs.fill_alpha as f32,
                     });
                }
            }
        }
        Ok(())
    }

    fn handle_bdc(&mut self, op: &Operation) -> PdfResult<()> {
        if op.operands.len() >= 2 && op.operands[0] == Object::new_name(b"OC".to_vec()) {
            let properties = &op.operands[1];
            let visible = if let Some(ref ctx) = self.oc_context {
                if let Some(ref res) = self.resources {
                     let actual_props = match properties {
                         Object::Name(n) => res.get_properties(n).unwrap_or(Object::Null),
                         _ => properties.clone(),
                     };
                     ctx.is_visible(&actual_props, res.resolver)
                } else {
                     true
                }
            } else {
                true
            };
            self.visibility_stack.push(self.current_visibility() && visible);
        } else {
            self.visibility_stack.push(self.current_visibility());
        }
        Ok(())
    }

    fn handle_emc(&mut self, _op: &Operation) -> PdfResult<()> {
        if self.visibility_stack.len() > 1 {
            self.visibility_stack.pop();
        }
        Ok(())
    }

    fn handle_bmc(&mut self, _op: &Operation) -> PdfResult<()> {
        self.visibility_stack.push(self.current_visibility());
        Ok(())
    }

    fn apply_ext_gstate(&mut self, dict: &BTreeMap<Vec<u8>, Object>) -> PdfResult<()> {
        debug_assert!(!dict.is_empty(), "apply_ext_gstate: dict empty");
        
        let bm_raw = dict.get(b"BM".as_ref()).and_then(|o| if let Object::Name(n) = o { Some(n.as_slice()) } else { None });
        let stroke_alpha = dict.get(b"CA".as_ref()).and_then(|o| self.extract_f64_operands(std::slice::from_ref(o), 1).first().copied());
        let fill_alpha = dict.get(b"ca".as_ref()).and_then(|o| self.extract_f64_operands(std::slice::from_ref(o), 1).first().copied());
        let smask = dict.get(b"SMask".as_ref()).cloned();
        let ais = dict.get(b"AIS".as_ref()).and_then(|o| if let Object::Boolean(b) = o { Some(*b) } else { None });
        let sa_val = dict.get(b"SA".as_ref()).and_then(|o| if let Object::Boolean(b) = o { Some(*b) } else { None });
        let bpc = dict.get(b"BPC".as_ref()).and_then(|o| if let Object::Boolean(b) = o { Some(*b) } else { None });

        let gs = self.gs_stack.current_mut()?;
        if let Some(bm_name) = bm_raw {
             gs.blend_mode = match bm_name {
                b"Multiply" => crate::graphics::BlendMode::Multiply,
                b"Screen" => crate::graphics::BlendMode::Screen,
                b"Overlay" => crate::graphics::BlendMode::Overlay,
                b"Darken" => crate::graphics::BlendMode::Darken,
                b"Lighten" => crate::graphics::BlendMode::Lighten,
                b"ColorDodge" => crate::graphics::BlendMode::ColorDodge,
                b"ColorBurn" => crate::graphics::BlendMode::ColorBurn,
                b"HardLight" => crate::graphics::BlendMode::HardLight,
                b"SoftLight" => crate::graphics::BlendMode::SoftLight,
                b"Difference" => crate::graphics::BlendMode::Difference,
                b"Exclusion" => crate::graphics::BlendMode::Exclusion,
                _ => crate::graphics::BlendMode::Normal,
            };
        }
        if let Some(sa) = stroke_alpha { gs.stroke_alpha = sa; }
        if let Some(fa) = fill_alpha { gs.fill_alpha = fa; }
        if let Some(sm) = smask { gs.soft_mask = Some(sm); }
        if let Some(ais_val) = ais { gs.alpha_source = ais_val; }
        if let Some(sa) = sa_val { gs.stroke_adjustment = sa; }
        if let Some(bpc_val) = bpc { gs.black_point_compensation = bpc_val; }

        debug_assert!(gs.stroke_alpha >= 0.0 && gs.stroke_alpha <= 1.0, "apply_ext_gstate: invalid stroke alpha");
        debug_assert!(gs.fill_alpha >= 0.0 && gs.fill_alpha <= 1.0, "apply_ext_gstate: invalid fill alpha");
        Ok(())
    }
}

/// Parses a PDF content stream into a structured tree of operations and blocks.
/// (Clause 7.8.2 - Content Streams)
pub fn parse_content_stream(input: &[u8]) -> PdfResult<Vec<ContentNode>> {
    let mut current_input = input;
    let mut root_nodes = Vec::new();
    let mut block_stack: Vec<(Vec<u8>, Vec<ContentNode>)> = Vec::new();
    let mut operand_stack: Vec<Object> = Vec::new();
    let mut loop_count = 0;
    const MAX_OPS: usize = 1_000_000;

    while !current_input.is_empty() {
        if let Ok((next_input, obj)) = crate::lexer::parse_object(current_input) {
            operand_stack.push(obj);
            current_input = next_input;
        } else if let Ok((next_input, op)) = crate::lexer::parse_operator(current_input) {
            let operands = std::mem::take(&mut operand_stack);
            handle_parsed_operator(op, operands, &mut block_stack, &mut root_nodes)?;
            current_input = next_input;
        } else if current_input.first().is_some_and(|&b| b.is_ascii_whitespace()) {
            current_input = &current_input[1..];
        } else {
            return Err(PdfError::ParseError(ParseErrorVariant::general(0, "Malformed content stream")));
        }
        loop_count += 1;
        if loop_count > MAX_OPS { return Err(PdfError::ContentError("Content stream operation limit exceeded".into())); }
    }
    if !block_stack.is_empty() { return Err(PdfError::ContentError("Unclosed graphics or text block".into())); }
    Ok(root_nodes)
}

fn handle_parsed_operator(
    op: Vec<u8>, 
    operands: Vec<Object>, 
    block_stack: &mut Vec<(Vec<u8>, Vec<ContentNode>)>,
    root_nodes: &mut Vec<ContentNode>
) -> PdfResult<()> {
    if op == b"q" || op == b"BT" {
        block_stack.push((op, Vec::new()));
    } else if op == b"Q" || op == b"ET" {
        let expected_start: &[u8] = if op == b"Q" { b"q" } else { b"BT" };
        let (start_op, children) = block_stack.pop().ok_or_else(|| {
            PdfError::ContentError(format!("Unexpected block end: {}", std::str::from_utf8(&op).unwrap_or("?")).into())
        })?;
        if start_op != expected_start {
            return Err(PdfError::ContentError(format!("Mismatched block end: expected {}, found {}", 
                std::str::from_utf8(expected_start).unwrap_or("?"), 
                std::str::from_utf8(&op).unwrap_or("?")).into()));
        }
        let block = ContentNode::Block(start_op, children);
        push_content_node(block, block_stack, root_nodes);
    } else {
        let operation = ContentNode::Operation(Operation { operands, operator: op });
        push_content_node(operation, block_stack, root_nodes);
    }
    Ok(())
}

fn push_content_node(
    node: ContentNode, 
    block_stack: &mut [(Vec<u8>, Vec<ContentNode>)],
    root_nodes: &mut Vec<ContentNode>
) {
    if let Some(parent) = block_stack.last_mut() {
        parent.1.push(node);
    } else {
        root_nodes.push(node);
    }
}
