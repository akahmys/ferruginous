use ferruginous_core::graphics::{
    GraphicsState, TextMatrices, WindingRule, Rect,
};
use ferruginous_core::{Object, PdfError, PdfName, PdfResult, Document, Handle};
use ferruginous_core::font::FontResource;
use ferruginous_core::lexer::Token;
use ferruginous_core::parser::Parser;
use ferruginous_render::{RenderBackend, path::PathBuilder};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Operators handling submodules.
pub mod ops;
/// Font resolution and rescue logic.
pub mod font;

/// A content stream interpreter that translates PDF operators into [RenderBackend] calls.
pub struct Interpreter<'a> {
    /// The rendering backend used to draw items.
    pub(crate) backend: &'a mut dyn RenderBackend,
    /// The document being interpreted.
    pub(crate) doc: &'a Document,
    /// Stack of resource dictionaries for hierarchical lookup (Form XObjects).
    pub(crate) resource_stack: Vec<Handle<BTreeMap<Handle<PdfName>, Object>>>,
    /// Operand stack for operators.
    pub(crate) stack: Vec<Object>,
    /// Current path being constructed.
    pub(crate) path: PathBuilder,
    /// Pending clipping rule from W or W* operator.
    pub(crate) pending_clip: Option<WindingRule>,
    /// Graphics state stack (managed by q/Q).
    pub(crate) state_stack: Vec<GraphicsState>,
    /// Current active graphics state.
    pub(crate) state: GraphicsState,
    /// Current text object state (managed by BT/ET).
    pub(crate) text_matrices: Option<TextMatrices>,
    /// Bounding box of the current text object (between BT and ET).
    pub current_text_bbox: Option<Rect>,
    /// Bounding box of all text objects combined on the page.
    pub page_text_bbox: Option<Rect>,
    /// Cache of fonts already defined in the backend.
    pub(crate) defined_fonts: BTreeSet<String>,
    pub(crate) font_cache: BTreeMap<Handle<Object>, Arc<FontResource>>,
    pub(crate) font_name_map: BTreeMap<Handle<Object>, String>,
    pub(crate) global_rescue_cmap: Option<ferruginous_core::font::cmap::CMap>,
}

impl<'a> Interpreter<'a> {
    /// Creates a new interpreter tied to a specific rendering backend.
    pub fn new(
        backend: &'a mut dyn RenderBackend,
        doc: &'a Document,
        initial_resources: Handle<BTreeMap<Handle<PdfName>, Object>>,
        initial_transform: kurbo::Affine,
    ) -> Self {
        let state = GraphicsState {
            ctm: ferruginous_core::graphics::Matrix(initial_transform.as_coeffs()),
            ..GraphicsState::default()
        };
        
        backend.transform(initial_transform);
        
        let mut interpreter = Self {
            backend,
            doc,
            resource_stack: vec![initial_resources],
            stack: Vec::new(),
            path: PathBuilder::new(),
            pending_clip: None,
            state_stack: Vec::new(),
            state,
            text_matrices: None,
            current_text_bbox: None,
            page_text_bbox: None,
            defined_fonts: BTreeSet::new(),
            font_cache: BTreeMap::new(),
            font_name_map: BTreeMap::new(),
            global_rescue_cmap: None,
        };
        
        interpreter.scan_for_global_rescue_cmap(initial_resources);
        interpreter
    }

    /// Executes a content stream by parsing and processing its operators.
    pub fn execute(&mut self, data: &[u8]) -> PdfResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        let mut parser = Parser::new(bytes::Bytes::copy_from_slice(data), self.doc.arena());

        while let Ok(token) = parser.peek() {
            if token == Token::EOF { break; }
            match token {
                Token::Keyword(ref op) => {
                    let op_str = op.clone();
                    
                    let _ = parser.next()?; // Consume operator
                    self.execute_operator(&op_str)?;
                }
                _ => {
                    let obj = parser.parse_object()?;
                    self.stack.push(obj);
                }
            }
        }
        Ok(())
    }

    fn execute_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "m" | "l" | "c" | "v" | "y" | "re" | "h" | "W" | "W*" => self.handle_path_operator(op)?,
            "S" | "f" | "F" | "f*" | "n" | "b" | "b*" | "B" | "B*" | "s" => self.handle_painting_operator(op)?,
            "q" | "Q" | "cm" | "gs" => self.handle_state_operator(op)?,
            "g" | "G" | "rg" | "RG" | "k" | "K" => self.handle_color_operator(op)?,
            "Tc" | "Tw" | "Tz" | "TL" | "Tf" | "Tr" | "Ts" => self.handle_text_state_operator(op)?,
            "BT" | "ET" => self.handle_text_scope_operator(op)?,
            "Td" | "TD" | "Tm" | "T*" => self.handle_text_positioning_operator(op)?,
            "Tj" | "TJ" | "'" | "\"" => self.handle_text_showing_operator(op)?,
            "Do" => self.handle_xobject_operator()?,
            "BMC" | "BDC" | "EMC" | "MP" | "DP" => self.handle_marked_content_operator(op)?,
            _ => {}
        }
        self.stack.clear();
        Ok(())
    }

    pub(crate) fn pop_i64(&mut self) -> PdfResult<i64> {
        match self.stack.pop() {
            Some(obj) => obj.as_integer().ok_or_else(|| PdfError::Other("Expected integer".into())),
            None => Err(PdfError::Other("Stack underflow".into())),
        }
    }

    pub(crate) fn pop_f64(&mut self) -> PdfResult<f64> {
        match self.stack.pop() {
            Some(obj) => obj.as_f64().ok_or_else(|| PdfError::Other("Expected number".into())),
            None => Err(PdfError::Other("Stack underflow".into())),
        }
    }

    pub(crate) fn pop_string(&mut self) -> PdfResult<bytes::Bytes> {
        match self.stack.pop() {
            Some(Object::String(s)) => Ok(s),
            _ => Err(PdfError::Other("Expected string".into())),
        }
    }

    pub(crate) fn pop_array(&mut self) -> PdfResult<Handle<Vec<Object>>> {
        match self.stack.pop() {
            Some(Object::Array(a)) => Ok(a),
            _ => Err(PdfError::Other("Expected array".into())),
        }
    }

    pub(crate) fn pop_name(&mut self) -> PdfResult<PdfName> {
        match self.stack.pop() {
            Some(Object::Name(h)) => {
                self.doc.arena().get_name(h).ok_or_else(|| PdfError::Other("Invalid name handle".into()))
            }
            _ => Err(PdfError::Other("Expected name".into())),
        }
    }

    pub(crate) fn find_resource(&self, res_type: &Handle<PdfName>, name: &PdfName) -> PdfResult<Object> {
        let res_type_key = *res_type;
        let name_handle = self.doc.arena().intern_name(name.clone());
        
        for res_handle in self.resource_stack.iter().rev() {
            let h = *res_handle;
            let dict = self.doc.arena().get_dict(h).ok_or_else(|| PdfError::Other("Invalid resource dict handle".into()))?;
                if let Some(entry) = dict.get(&res_type_key).and_then(|o| o.resolve(self.doc.arena()).as_dict_handle()) {
                    let res_dict = self.doc.arena().get_dict(entry).ok_or_else(|| PdfError::Other("Invalid resource type dict".into()))?;
                    if let Some(res) = res_dict.get(&name_handle) {
                        return Ok(res.clone());
                    }
                }
            }
        
        let type_name = self.doc.arena().get_name(res_type_key).map(|n| n.as_str().to_string()).unwrap_or_default();
        Err(PdfError::Other(format!("Resource not found: {} /{}", type_name, name.as_str())))
    }
}
