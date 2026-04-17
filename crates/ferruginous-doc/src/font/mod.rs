use std::sync::Arc;
use ferruginous_core::PdfName;
use crate::font::cmap::CMap;

pub mod cmap;

/// ISO 32000-2:2020 Clause 9.8 - Font Descriptors
#[derive(Debug, Clone)]
pub struct FontDescriptor {
    pub font_name: PdfName,
    pub flags: i32,
    pub font_bbox: [f64; 4],
    pub italic_angle: f64,
    pub ascent: f64,
    pub descent: f64,
    pub cap_height: f64,
    pub stem_v: f64,
    pub missing_width: f64,
    pub font_file: Option<Vec<u8>>, // Extracted from /FontFile, /FontFile2, or /FontFile3
}

/// Represents a PDF Font resource.
/// (ISO 32000-2:2020 Clause 9.6 and 9.7)
#[derive(Debug, Clone)]
pub enum FontResource {
    Simple(SimpleFont),
    Composite(CompositeFont),
}

#[derive(Debug, Clone)]
pub struct SimpleFont {
    pub subtype: PdfName, // /Type1, /TrueType, /Type3
    pub base_font: PdfName,
    pub first_char: u8,
    pub last_char: u8,
    pub widths: Vec<f64>,
    pub descriptor: Option<FontDescriptor>,
    pub encoding: Option<String>, // Standard encoding name
    pub to_unicode: Option<Arc<CMap>>,
}

#[derive(Debug, Clone)]
pub struct CompositeFont {
    pub subtype: PdfName, // /Type0
    pub base_font: PdfName,
    pub encoding: Arc<CMap>,
    pub descendant_fonts: Vec<FontResource>, // Usually contains one CIDFont
    pub to_unicode: Option<Arc<CMap>>,
}

impl FontResource {
    pub fn base_font(&self) -> &PdfName {
        match self {
            Self::Simple(f) => &f.base_font,
            Self::Composite(f) => &f.base_font,
        }
    }

    /// Returns the width of a character glyph in text space units.
    pub fn glyph_width(&self, code: &[u8]) -> f64 {
        match self {
            Self::Simple(f) => {
                if code.len() == 1 {
                    let c = code[0];
                    if c >= f.first_char && c <= f.last_char {
                        let idx = (c - f.first_char) as usize;
                        if idx < f.widths.len() {
                            return f.widths[idx];
                        }
                    }
                }
                f.descriptor.as_ref().map(|d| d.missing_width).unwrap_or(0.0)
            }
            Self::Composite(_) => {
                // CIDFont width lookup logic goes here
                // For now, return a default
                1000.0
            }
        }
    }
}
