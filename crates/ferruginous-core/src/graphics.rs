//! PDF Graphics State Constants & Types (ISO 32000-2:2020 Clause 8)

use serde::{Serialize, Deserialize};
use kurbo::Affine;

/// PDF Color representation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Color {
    Gray(f64),
    Rgb(f64, f64, f64),
    Cmyk(f64, f64, f64, f64),
}

/// Standard PDF Blend Modes (ISO 32000-2 Table 141)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

/// Path Winding Rules (ISO 32000-2 Clause 8.5.3.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindingRule {
    NonZero,
    EvenOdd,
}

/// Line Cap Styles (ISO 32000-2 Table 53)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// Line Join Styles (ISO 32000-2 Table 54)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// Stroke Style Parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    pub dash_pattern: Option<(Vec<f64>, f64)>,
}

/// Image Pixel Formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Gray8,
    Rgb8,
    Cmyk8,
}

/// Standard PDF 2D Transformation Matrix (ISO 32000-2 Clause 8.3.3)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Matrix(pub [f64; 6]);

impl Default for Matrix {
    fn default() -> Self {
        Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    }
}

impl Matrix {
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self([a, b, c, d, e, f])
    }

    pub fn as_affine(&self) -> Affine {
        Affine::new(self.0)
    }

    pub fn concat(&self, other: &Self) -> Self {
        let res = self.as_affine() * other.as_affine();
        Self(res.as_coeffs())
    }
}

/// A simple axis-aligned rectangle (ISO 32000-2 Clause 7.3.6)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Rect {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }
}

/// Graphics State Parameters (ISO 32000-2 Table 52)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsState {
    pub ctm: Matrix,
    pub stroke_color: Color,
    pub fill_color: Color,
    pub stroke_style: StrokeStyle,
    pub fill_alpha: f64,
    pub stroke_alpha: f64,
    pub blend_mode: BlendMode,
    pub text_state: TextState,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::default(),
            stroke_color: Color::Gray(0.0),
            fill_color: Color::Gray(0.0),
            stroke_style: StrokeStyle {
                width: 1.0,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                miter_limit: 10.0,
                dash_pattern: None,
            },
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend_mode: BlendMode::Normal,
            text_state: TextState::default(),
        }
    }
}

/// Text State Parameters (ISO 32000-2 Table 105)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextState {
    pub char_spacing: f64,
    pub word_spacing: f64,
    pub horizontal_scaling: f64,
    pub leading: f64,
    pub font: Option<crate::object::PdfName>,
    pub font_size: f64,
    pub rendering_mode: TextRenderingMode,
    pub rise: f64,
    pub knockout: bool,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            font: None,
            font_size: 1.0,
            rendering_mode: TextRenderingMode::Fill,
            rise: 0.0,
            knockout: true,
        }
    }
}

/// Text Rendering Modes (ISO 32000-2 Table 106)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextRenderingMode {
    Fill = 0,
    Stroke = 1,
    FillStroke = 2,
    Invisible = 3,
    FillClip = 4,
    StrokeClip = 5,
    FillStrokeClip = 6,
    Clip = 7,
}

impl From<i64> for TextRenderingMode {
    fn from(val: i64) -> Self {
        match val {
            0 => Self::Fill,
            1 => Self::Stroke,
            2 => Self::FillStroke,
            3 => Self::Invisible,
            4 => Self::FillClip,
            5 => Self::StrokeClip,
            6 => Self::FillStrokeClip,
            7 => Self::Clip,
            _ => Self::Fill,
        }
    }
}

/// Text Object Matrices (BT/ET Scope)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TextMatrices {
    pub tm: Matrix,
    pub tlm: Matrix,
}
