//! Graphics state and path construction management.
//! (ISO 32000-2:2020 Clause 8.4)

use crate::core::Object;
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
    Pattern(Option<Vec<u8>>),
}

impl Color {
    /// Converts this color to RGB (0.0 to 1.0) given the active color space.
    pub fn to_rgb(&self, space: &crate::colorspace::ColorSpace) -> [f32; 3] {
        match self {
            Self::Gray(g) => [*g, *g, *g],
            Self::RGB(r, g, b) => [*r, *g, *b],
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
}

impl Shading {
    /// Creates a Shading instance from a dictionary.
    pub fn from_dict(dict: &BTreeMap<Vec<u8>, Object>, resolver: &dyn crate::core::Resolver) -> crate::core::PdfResult<Self> {
        let st = match dict.get(b"ShadingType".as_ref()) {
            Some(Object::Integer(i)) => ShadingType::from_int(*i as i32),
            _ => ShadingType::Axial,
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
pub enum BlendMode {
    /// Normal (default).
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

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// A glyph instance with its position and bounding box.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GlyphInstance {
    /// Glyph ID or character code.
    pub char_code: Vec<u8>,
    /// Horizontal advance (glyph space).
    pub x_advance: f64,
    /// Bounding box in page space.
    pub bbox: Rect,
    /// Actual glyph outline path (in page space).
    pub path: Option<Arc<BezPath>>,
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
}

/// ISO 32000-2:2020 Clause 8.4.2 - Graphics State
/// Contains parameters that control the appearance of graphics and text on a page.
#[derive(Debug, Clone, PartialEq)]
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
        
        gss.push().unwrap();
        assert_eq!(gss.current().expect("test").line_width, 2.0);
        
        gss.current_mut().expect("test").line_width = 3.0;
        assert_eq!(gss.current().expect("test").line_width, 3.0);
        
        gss.pop().unwrap();
        assert_eq!(gss.current().expect("test").line_width, 2.0);
    }
}
