//! Graphics state and path construction management.
//!
//! (ISO 32000-2:2020 Clause 8.4)

use crate::core::{Object, Reference};
pub use kurbo::{Affine, BezPath, Rect};
use serde::Serialize;
use std::sync::Arc;
use std::collections::BTreeMap;

pub mod query;

/// Clipping rule for determining the interior of a path (Clause 8.5.3.3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ClippingRule {
    /// Non-zero winding number rule (W).
    NonZeroWinding,
    /// Even-odd rule (W*).
    EvenOdd,
}

/// Represents a color in various color spaces (Clause 8.6).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Color {
    /// Grayscale (g/G).
    Gray(f32),
    /// RGB color space (rg/RG).
    RGB(f32, f32, f32),
    /// CMYK color space (k/K).
    CMYK(f32, f32, f32, f32),
    /// High-precision components for ICCBased/Calibrated spaces.
    ICC(Vec<f32>),
    /// Lab components.
    Lab(f32, f32, f32),
    /// Specialized color value (e.g. for Separation/DeviceN).
    Special(Vec<f32>),
    /// Pattern color.
    Pattern(Option<Arc<TilingPattern>>),
}

impl Color {
    /// Converts this color to RGB (0.0 to 1.0) given the active color space.
    #[allow(clippy::many_single_char_names)]
    pub fn to_rgb(&self, space: &crate::colorspace::ColorSpace) -> [f32; 3] {
        match self {
            Self::Gray(g) => [*g, *g, *g],
            Self::RGB(r, g, b) => [*r, *g, *b],
            #[allow(clippy::many_single_char_names)]
            Self::CMYK(c, m, y, k) => {
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                [r, g, b]
            }
            Self::ICC(components) | Self::Special(components) => space.to_rgb(components),
            Self::Lab(l, a, b) => space.to_rgb(&[*l, *a, *b]),
            Self::Pattern(_) => [0.0, 0.0, 0.0], // Patterns handle their own rendering
        }
    }
}

/// Represents a Shading object (Clause 8.7.4.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shading {
    /// Shading type (1-7).
    pub shading_type: ShadingType,
    /// Color space in which shading is performed.
    pub color_space: crate::colorspace::ColorSpace,
    /// Background color to be used prior to shading.
    pub background: Option<Color>,
    /// Bounding box in the shading's coordinate system.
    pub bbox: Option<kurbo::Rect>,
    /// Anti-aliasing flag.
    pub anti_alias: bool,
    /// Coordinates [x0 y0 x1 y1] for Axial (Type 2) or [x0 y0 r0 x1 y1 r1] for Radial (Type 3).
    pub coords: Vec<f64>,
    /// Function object(s) defining the color gradient.
    pub function: Arc<Vec<Object>>,
    /// Extend flags (extend at start, extend at end).
    pub extend: [bool; 2],
}

/// Represents a Tiling Pattern (Clause 8.7.3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TilingPattern {
    /// Paint type: 1 (colored), 2 (uncolored).
    pub paint_type: i32,
    /// Tiling type: 1, 2, or 3.
    pub tiling_type: i32,
    /// Bounding box in the pattern's coordinate system.
    pub bbox: kurbo::Rect,
    /// Horizontal step between tiles.
    pub x_step: f32,
    /// Vertical step between tiles.
    pub y_step: f32,
    /// Transformation from pattern space to the shading's coordinate system.
    pub matrix: kurbo::Affine,
    /// Resources used by the pattern's content stream.
    pub resources: Option<Object>,
    /// The pattern's content stream data.
    pub data: Arc<Vec<u8>>,
}

impl TilingPattern {
    /// Creates a TilingPattern from a stream object.
    pub fn from_stream(dict: &BTreeMap<Vec<u8>, Object>, data: Arc<Vec<u8>>) -> Self {
        let paint_type = match dict.get(b"PaintType".as_ref()) {
            Some(Object::Integer(i)) => *i as i32,
            _ => 1,
        };
        let tiling_type = match dict.get(b"TilingType".as_ref()) {
            Some(Object::Integer(i)) => *i as i32,
            _ => 1,
        };
        let bbox = match dict.get(b"BBox".as_ref()) {
            Some(Object::Array(arr)) if arr.len() == 4 => {
                let v: Vec<f64> = arr.iter().filter_map(|o| if let Object::Real(f) = o { Some(*f) } else if let Object::Integer(i) = o { Some(*i as f64) } else { None }).collect();
                if v.len() == 4 { kurbo::Rect::new(v[0], v[1], v[2], v[3]) } else { kurbo::Rect::new(0.0, 0.0, 100.0, 100.0) }
            }
            _ => kurbo::Rect::new(0.0, 0.0, 100.0, 100.0),
        };
        let x_step = match dict.get(b"XStep".as_ref()) {
            Some(Object::Real(f)) => *f as f32,
            Some(Object::Integer(i)) => *i as f32,
            _ => 100.0,
        };
        let y_step = match dict.get(b"YStep".as_ref()) {
            Some(Object::Real(f)) => *f as f32,
            Some(Object::Integer(i)) => *i as f32,
            _ => 100.0,
        };
        let matrix = match dict.get(b"Matrix".as_ref()) {
            Some(Object::Array(arr)) if arr.len() == 6 => {
                let v: Vec<f64> = arr.iter().filter_map(|o| if let Object::Real(f) = o { Some(*f) } else if let Object::Integer(i) = o { Some(*i as f64) } else { None }).collect();
                if v.len() == 6 { kurbo::Affine::new([v[0], v[1], v[2], v[3], v[4], v[5]]) } else { kurbo::Affine::IDENTITY }
            }
            _ => kurbo::Affine::IDENTITY,
        };
        let resources = dict.get(b"Resources".as_ref()).cloned();

        Self {
            paint_type,
            tiling_type,
            bbox,
            x_step,
            y_step,
            matrix,
            resources,
            data,
        }
    }
}

impl Shading {
    /// Creates a Shading instance from a dictionary.
    pub fn from_dict(dict: &BTreeMap<Vec<u8>, Object>, resolver: &dyn crate::core::Resolver) -> crate::core::PdfResult<Self> {
        let st = match dict.get(b"ShadingType".as_ref()) {
            Some(Object::Integer(i)) => ShadingType::from_int(*i as i32),
            _ => ShadingType::Axial,
        };

        let coords = match dict.get(b"Coords".as_ref()) {
            Some(Object::Array(arr)) => arr.iter().filter_map(|o| if let Object::Real(f) = o { Some(*f) } else if let Object::Integer(i) = o { Some(*i as f64) } else { None }).collect(),
            _ => Vec::new(),
        };

        let function = match dict.get(b"Function".as_ref()) {
            Some(Object::Array(arr)) => arr.clone(),
            Some(obj) => Arc::new(vec![obj.clone()]),
            _ => Arc::new(Vec::new()),
        };

        let extend = match dict.get(b"Extend".as_ref()) {
            Some(Object::Array(arr)) if arr.len() >= 2 => {
                let e0 = if let Object::Boolean(b) = arr[0] { b } else { false };
                let e1 = if let Object::Boolean(b) = arr[1] { b } else { false };
                [e0, e1]
            }
            _ => [false, false],
        };

        let cs = match dict.get(b"ColorSpace".as_ref()) {
            Some(obj) => crate::colorspace::ColorSpace::from_object(obj, resolver)?,
            _ => crate::colorspace::ColorSpace::DeviceRGB,
        };

        Ok(Self {
            shading_type: st,
            color_space: cs,
            background: None,
            bbox: None,
            anti_alias: false,
            coords,
            function,
            extend,
        })
    }
}

/// PDF Shading Types (ISO 32000-2 Table 78).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ShadingType {
    /// Function-based shading (Type 1).
    FunctionBased = 1,
    /// Axial shading (Type 2).
    Axial = 2,
    /// Radial shading (Type 3).
    Radial = 3,
    /// Free-form Gouraud-shaded triangle mesh (Type 4).
    FreeFormGouraud = 4,
    /// Lattice-form Gouraud-shaded triangle mesh (Type 5).
    LatticeFormGouraud = 5,
    /// Coons patch mesh (Type 6).
    CoonsPatch = 6,
    /// Tensor-product patch mesh (Type 7).
    TensorProductPatch = 7,
}

impl ShadingType {
    /// Creates a rendering operation from an integer value.
    pub fn from_int(i: i32) -> Self {
        match i {
            1 => Self::FunctionBased,
            2 => Self::Axial,
            3 => Self::Radial,
            4 => Self::FreeFormGouraud,
            5 => Self::LatticeFormGouraud,
            6 => Self::CoonsPatch,
            7 => Self::TensorProductPatch,
            _ => Self::Axial,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::Gray(0.0)
    }
}

/// PDF 2.0 Blend Modes (Clause 11.3.5, Table 138-139).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[derive(Default)]
pub enum BlendMode {
    /// Normal (default).
    #[default]
    Normal,
    /// Multiply.
    Multiply,
    /// Screen.
    Screen,
    /// Overlay.
    Overlay,
    /// Darken.
    Darken,
    /// Lighten.
    Lighten,
    /// ColorDodge.
    ColorDodge,
    /// ColorBurn.
    ColorBurn,
    /// HardLight.
    HardLight,
    /// SoftLight.
    SoftLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
}


/// A glyph instance with its position and bounding box.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GlyphInstance {
    /// Glyph ID or character code.
    pub char_code: Vec<u8>,
    /// Individual glyph position in page space.
    pub point: kurbo::Point,
    /// Horizontal advance (glyph space).
    pub x_advance: f64,
    /// Bounding box in page space.
    pub bbox: Rect,
    /// Actual glyph outline path (relative to point).
    pub path: Option<Arc<BezPath>>,
}

/// A single drawing command, combining an operation with its metadata (e.g., OCG layer).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DrawCommand {
    /// The actual drawing operation.
    pub op: DrawOp,
    /// Optional reference to the Optional Content Group (OCG) this command belongs to.
    pub oc: Option<Reference>,
}

/// A high-level drawing operation for the rendering bridge.
/// (ISO 32000-2:2020 Clause 8.4)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DrawOp {
    /// Save graphics state (q).
    PushState,
    /// Restore graphics state (Q).
    PopState,
    /// Set transformation matrix (cm).
    SetTransform(Affine),
    /// Fill the current path (f/f*/b/b*).
    FillPath { 
        /// The path to fill.
        path: Arc<BezPath>, 
        /// Filling color.
        color: Color, 
        /// Clipping rule.
        rule: ClippingRule,
        /// Blend mode for this operation.
        blend_mode: BlendMode,
        /// Alpha transparency (0.0 - 1.0).
        alpha: f32,
    },
    /// Stroke the current path (S/s/b/b*).
    StrokePath { 
        /// The path to stroke.
        path: Arc<BezPath>, 
        /// Stroking color.
        color: Color, 
        /// Line width.
        width: f64,
        /// Line cap style.
        line_cap: i32,
        /// Line join style.
        line_join: i32,
        /// Miter limit.
        miter_limit: f64,
        /// Dash pattern [array, phase].
        dash_pattern: (Vec<f64>, f64),
        /// Blend mode for this operation.
        blend_mode: BlendMode,
        /// Alpha transparency (0.0 - 1.0).
        alpha: f32,
    },
    /// Draw text (Tj/TJ).
    DrawText { 
        /// List of glyphs and their advances.
        glyphs: Vec<GlyphInstance>, 
        /// Resource name of the font.
        font_id: Vec<u8>, 
        /// Font size.
        size: f64,
        /// Filling color for text.
        color: Color,
        /// Blend mode for this operation.
        blend_mode: BlendMode,
        /// Alpha transparency (0.0 - 1.0).
        alpha: f32,
    },
    /// Draw an external object (Do).
    DrawImage { 
        /// Decoded pixel data.
        data: Arc<Vec<u8>>,
        /// Image width.
        width: u32,
        /// Image height.
        height: u32,
        /// Color space / Components (3 for RGB, 1 for Gray).
        components: u8,
        /// Bounding box in user space.
        rect: Rect,
        /// Blend mode for this operation.
        blend_mode: BlendMode,
        /// Alpha transparency (0.0 - 1.0).
        alpha: f32,
    },
    /// Fill a path with a shading (sh).
    DrawShading {
        /// The shading pattern to apply.
        shading: Arc<Shading>,
        /// Blend mode for this operation.
        blend_mode: BlendMode,
        /// Alpha transparency (0.0 - 1.0).
        alpha: f32,
    },
    /// Draw a complex path with specific outline (new in Phase 11).
    DrawPath(Arc<BezPath>, Color, f64),
    /// Intersect the current clipping path (W/W*).
    Clip(Arc<BezPath>, ClippingRule),
    /// Push a transparency group layer.
    PushLayer {
        /// Group attributes (Isolated/Knockout).
        attrs: GroupAttributes,
        /// Blend mode for the entire group.
        blend_mode: BlendMode,
        /// Group-level alpha.
        alpha: f32,
    },
    /// Pop the current transparency group layer.
    PopLayer,
}

/// Attributes for Transparency Groups (ISO 32000-2 Clause 11.4.7).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroupAttributes {
    /// Is the group isolated from its backdrop?
    pub isolated: bool,
    /// Is the group a knockout group?
    pub knockout: bool,
    /// Group's color space.
    pub color_space: Option<crate::colorspace::ColorSpace>,
}

/// ISO 32000-2:2020 Clause 8.4.2 - Graphics State
///
/// Contains parameters that control the appearance of graphics and text on a page.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct GraphicsState {
    /// Current Transformation Matrix (Clause 8.3.3).
    pub ctm: Affine,
    /// Line width in user space (Clause 8.4.3.2).
    pub line_width: f64,
    /// Line cap style (Clause 8.4.3.3).
    pub line_cap: i32,
    /// Line join style (Clause 8.4.3.4).
    pub line_join: i32,
    /// Miter limit (Clause 8.4.3.5).
    pub miter_limit: f64,
    /// Dash pattern [array, phase] (Clause 8.4.3.6).
    pub dash_pattern: (Vec<f64>, f64),
    /// Rendering intent (Clause 8.6.5.8).
    pub rendering_intent: Vec<u8>,
    /// Flatness tolerance (Clause 10.2).
    pub flatness: f64,
    /// Current path being constructed (Clause 8.5.2.1).
    pub current_path: Arc<BezPath>,
    /// Clipping path (the intersection of all clipping paths) (Clause 8.5.4).
    pub clipping_path: Arc<BezPath>,
    /// Pending clipping rule defined by W or W*.
    pub pending_clipping_rule: Option<ClippingRule>,
    // Color State (Clause 8.6)
    /// Current stroking color space.
    pub stroke_color_space: crate::colorspace::ColorSpace,
    /// Current filling color space.
    pub fill_color_space: crate::colorspace::ColorSpace,
    /// Current stroking color.
    pub stroke_color: Color,
    /// Current filling color.
    pub fill_color: Color,
    
    // Transparency Parameters (Clause 11)
    /// Blend mode (BM keyword).
    pub blend_mode: BlendMode,
    /// Stroke alpha (CA keyword).
    pub stroke_alpha: f64,
    /// Fill alpha (ca keyword).
    pub fill_alpha: f64,
    /// Soft mask dictionary/stream (`SMask` keyword).
    pub soft_mask: Option<Object>,
    /// Alpha source (AIS keyword).
    pub alpha_source: bool,
    /// Stroke adjustment (SA keyword).
    pub stroke_adjustment: bool,

    // ISO 32000-2 Table 51 - Device-Dependent Parameters
    /// Overprint mode (OPM keyword).
    pub overprint_mode: i32,
    /// Overprint for stroking (OP keyword).
    pub overprint_stroke: bool,
    /// Overprint for non-stroking (op keyword).
    pub overprint_fill: bool,
    /// Smoothness tolerance (sm keyword).
    pub smoothness: f64,
    /// Halftone dictionary/stream/name (HT keyword).
    pub halftone: Option<Object>,
    /// Transfer function (TR/TR2 keyword).
    pub transfer_function: Option<Object>,

    /// Black Point Compensation (ISO 32000-2 Clause 8.6.5.9).
    pub black_point_compensation: bool,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Affine::IDENTITY,
            line_width: 1.0,
            line_cap: 0,
            line_join: 0,
            miter_limit: 10.0,
            dash_pattern: (Vec::new(), 0.0),
            rendering_intent: b"RelativeColorimetric".to_vec(),
            flatness: 1.0,
            current_path: Arc::new(BezPath::new()),
            clipping_path: Arc::new(BezPath::new()),
            pending_clipping_rule: None,
            stroke_color_space: crate::colorspace::ColorSpace::DeviceGray,
            fill_color_space: crate::colorspace::ColorSpace::DeviceGray,
            stroke_color: Color::Gray(0.0),
            fill_color: Color::Gray(0.0),
            blend_mode: BlendMode::Normal,
            stroke_alpha: 1.0,
            fill_alpha: 1.0,
            soft_mask: None,
            alpha_source: false,
            stroke_adjustment: false,
            // Table 51 Defaults
            overprint_mode: 0,
            overprint_stroke: false,
            overprint_fill: false,
            smoothness: 0.0,
            halftone: None,
            transfer_function: None,
            black_point_compensation: false,
        }
    }
}

/// Represents the stack of graphics states (q and Q operators).
/// (ISO 32000-2:2020 Clause 8.4.2)
pub struct GraphicsStateStack {
    /// Internal vector representing the LIFO stack.
    stack: Vec<GraphicsState>,
}

impl Default for GraphicsStateStack {
    /// Creates a new `GraphicsStateStack` with a default initial state.
    fn default() -> Self {
        Self::new()
    }
}

impl GraphicsStateStack {
    /// Creates a new stack with a default initial state.
    #[must_use] pub fn new() -> Self {
        Self {
            stack: vec![GraphicsState::default()],
        }
    }

    /// Returns a reference to the top state on the stack.
    ///
    /// # Errors
    /// Returns an error if the stack is unexpectedly empty.
    pub fn current(&self) -> crate::core::error::PdfResult<&GraphicsState> {
        self.stack.last().ok_or_else(|| crate::core::error::PdfError::ContentError("Graphics state stack is empty".into()))
    }

    /// Returns a mutable reference to the top state on the stack.
    ///
    /// # Errors
    /// Returns an error if the stack is unexpectedly empty.
    pub fn current_mut(&mut self) -> crate::core::error::PdfResult<&mut GraphicsState> {
        self.stack.last_mut().ok_or_else(|| crate::core::error::PdfError::ContentError("Graphics state stack is empty".into()))
    }

    /// Clause 8.4.2 - q (save). Pushes a clone of the current state.
    /// (ISO 32000-2:2020 Clause 8.4.2)
    ///
    /// # Errors
    /// Returns an error if the stack depth limit (32) is exceeded.
    pub fn push(&mut self) -> crate::core::error::PdfResult<()> {
        if self.stack.len() >= 32 {
            return Err(crate::core::error::PdfError::ContentError("Graphics state stack nesting depth exceeded (max 32)".into()));
        }
        let current = self.current()?.clone();
        self.stack.push(current);
        Ok(())
    }

    /// Clause 8.4.2 - Q (restore). Pops the current state.
    /// (ISO 32000-2:2020 Clause 8.4.2)
    ///
    /// # Errors
    /// Returns an error if the stack underflow occurs.
    pub fn pop(&mut self) -> crate::core::error::PdfResult<()> {
        if self.stack.len() <= 1 {
            return Err(crate::core::error::PdfError::ContentError("Graphics state stack underflow".into()));
        }
        self.stack.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gs_stack_push_pop() {
        let mut gss = GraphicsStateStack::new();
        gss.current_mut().expect("test").line_width = 2.0;
        
        gss.push().expect("push failed");
        const EPSILON: f64 = 0.000_1;
        assert!((gss.current().expect("test").line_width - 2.0).abs() < EPSILON);
        
        gss.current_mut().expect("test").line_width = 3.0;
        assert!((gss.current().expect("test").line_width - 3.0).abs() < EPSILON);
        
        gss.pop().expect("pop failed");
        assert!((gss.current().expect("test").line_width - 2.0).abs() < EPSILON);
    }
}
