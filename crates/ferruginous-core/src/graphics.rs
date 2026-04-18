use serde::{Deserialize, Serialize};
use kurbo::Affine;

/// ISO 32000-2:2020 Clause 8.3.2 - Transformation Matrices
///
/// A transformation matrix specifies the relationship between two coordinate spaces.
/// In PDF, transformation matrices are $3 \times 2$ affine matrices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix(pub Affine);

impl Matrix {
    /// The identity matrix [1 0 0 1 0 0].
    pub const IDENTITY: Self = Self(Affine::IDENTITY);

    /// Creates a new matrix [a b c d e f].
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self(Affine::new([a, b, c, d, e, f]))
    }

    /// Multiplies two matrices.
    pub fn concat(&self, other: &Self) -> Self {
        Self(self.0 * other.0)
    }
}

/// ISO 32000-2:2020 Clause 8.6.2 - Color Spaces
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Color {
    /// DeviceGray (Clause 8.6.4.2)
    Gray(f64),
    /// DeviceRGB (Clause 8.6.4.3)
    Rgb(f64, f64, f64),
    /// DeviceCMYK (Clause 8.6.4.4)
    Cmyk(f64, f64, f64, f64),
}

impl Default for Color {
    fn default() -> Self {
        Self::Gray(0.0) // Default to Black
    }
}

/// ISO 32000-2:2020 Clause 7.3.6 - Rectangles
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

    pub fn width(&self) -> f64 {
        (self.x2 - self.x1).abs()
    }

    pub fn height(&self) -> f64 {
        (self.y2 - self.y1).abs()
    }

    /// Extends the rectangle to include another rectangle.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }
}

/// ISO 32000-2:2020 Clause 9.3 - Text State Parameters
#[derive(Debug, Clone, PartialEq)]
pub struct TextState {
    pub char_spacing: f64,    // Tc
    pub word_spacing: f64,    // Tw
    pub horizontal_scaling: f64, // Th (100.0 is normal)
    pub leading: f64,         // Tl
    pub font: Option<crate::PdfName>, // Tf (font name)
    pub font_size: f64,       // Tfs
    pub rendering_mode: TextRenderingMode, // Tmode
    pub rise: f64,            // Trise
    pub knockout: bool,       // Tknockout
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

/// ISO 32000-2:2020 Clause 8.4 - Graphics State
///
/// Encapsulates all state parameters that are saved/restored via q/Q operators.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsState {
    pub ctm: Matrix,
    pub stroke_style: StrokeStyle,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub text_state: TextState,
    pub winding_rule: WindingRule,
    pub fill_alpha: f64,   // ca
    pub stroke_alpha: f64, // CA
    pub blend_mode: BlendMode, // BM
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::IDENTITY,
            stroke_style: StrokeStyle::default(),
            fill_color: Color::Gray(0.0),
            stroke_color: Color::Gray(0.0),
            text_state: TextState::default(),
            winding_rule: WindingRule::NonZero,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend_mode: BlendMode::Normal,
        }
    }
}

/// ISO 32000-2:2020 Clause 9.3.6 - Text Rendering Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// ISO 32000-2:2020 Clause 9.4.2 - Text Matrices
///
/// Manages the text matrix (Tm) and line matrix (Tlm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMatrices {
    pub tm: Matrix,  // Text matrix
    pub tlm: Matrix, // Text line matrix
}

impl Default for TextMatrices {
    fn default() -> Self {
        Self {
            tm: Matrix::IDENTITY,
            tlm: Matrix::IDENTITY,
        }
    }
}

impl TextMatrices {
    /// Returns the combined matrix that transforms text space to user space (Tm * CTM).
    pub fn text_to_user_space(&self, ctm: &Matrix) -> Matrix {
        self.tm.concat(ctm)
    }
}

/// ISO 32000-2:2020 Clause 8.5.3 - Winding Rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindingRule {
    NonZero, // W
    EvenOdd, // W*
}

/// ISO 32000-2:2020 Clause 8.4.3.4 - Line Cap Style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    Butt = 0,
    Round = 1,
    Square = 2,
}

impl From<i64> for LineCap {
    fn from(val: i64) -> Self {
        match val {
            0 => Self::Butt,
            1 => Self::Round,
            2 => Self::Square,
            _ => Self::Butt,
        }
    }
}

/// ISO 32000-2:2020 Clause 8.4.3.5 - Line Join Style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

impl From<i64> for LineJoin {
    fn from(val: i64) -> Self {
        match val {
            0 => Self::Miter,
            1 => Self::Round,
            2 => Self::Bevel,
            _ => Self::Miter,
        }
    }
}

/// ISO 32000-2:2020 Clause 8.4.3 - Graphics State Parameters (Stroke)
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeStyle {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    pub dash_pattern: Option<(Vec<f64>, f64)>, // (dash_array, dash_phase)
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_pattern: None,
        }
    }
}

/// Supported pixel formats for Image XObjects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    /// 8-bit Gray (1 byte per pixel)
    Gray8,
    /// 8-bit RGB (3 bytes per pixel)
    Rgb8,
    /// 8-bit CMYK (4 bytes per pixel)
    Cmyk8,
}

/// ISO 32000-2:2020 Clause 11.3.5 - Blend Modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
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

impl std::str::FromStr for BlendMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Multiply" => Self::Multiply,
            "Screen" => Self::Screen,
            "Overlay" => Self::Overlay,
            "Darken" => Self::Darken,
            "Lighten" => Self::Lighten,
            "ColorDodge" => Self::ColorDodge,
            "ColorBurn" => Self::ColorBurn,
            "HardLight" => Self::HardLight,
            "SoftLight" => Self::SoftLight,
            "Difference" => Self::Difference,
            "Exclusion" => Self::Exclusion,
            "Hue" => Self::Hue,
            "Saturation" => Self::Saturation,
            "Color" => Self::Color,
            "Luminosity" => Self::Luminosity,
            _ => Self::Normal,
        })
    }
}
