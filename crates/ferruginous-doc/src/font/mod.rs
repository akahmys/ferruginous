use std::sync::Arc;
use std::collections::BTreeMap;
use ferruginous_core::{Object, PdfName, PdfResult, PdfError, Resolver};
use crate::font::cmap::{CMap, MappingResult};

pub mod cmap;
pub mod encoding;
pub mod standard;

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
    pub font_file: Option<Vec<u8>>,
}

/// Consolidated metrics for a font.
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    pub ascent: f64,
    pub descent: f64,
    pub cap_height: f64,
    pub bbox: [f64; 4],
}

/// Represents CID-keyed width information (ISO 32000-2:2020 Clause 9.7.4.3)
#[derive(Debug, Clone, Default)]
pub struct CIDWidths {
    pub default_width: f64,
    pub widths: BTreeMap<u32, f64>,
}

impl CIDWidths {
    pub fn get(&self, cid: u32) -> f64 {
        self.widths.get(&cid).copied().unwrap_or(self.default_width)
    }
}

/// Represents a CIDFont (Descendant of a Type0 font).
#[derive(Debug, Clone)]
pub struct CIDFont {
    pub subtype: PdfName, // /CIDFontType0 or /CIDFontType2
    pub base_font: PdfName,
    pub cid_system_info: BTreeMap<PdfName, Object>,
    pub widths: CIDWidths,
    pub descriptor: FontDescriptor,
}

/// Represents a PDF Font resource.
#[derive(Debug, Clone)]
pub enum FontResource {
    Simple(SimpleFont),
    Composite(CompositeFont),
    CID(CIDFont),
}

#[derive(Debug, Clone)]
pub struct SimpleFont {
    pub subtype: PdfName,
    pub base_font: PdfName,
    pub first_char: u8,
    pub last_char: u8,
    pub widths: Vec<f64>,
    pub descriptor: Option<FontDescriptor>,
    pub encoding: Option<encoding::Encoding>,
    pub to_unicode: Option<Arc<CMap>>,
}

#[derive(Debug, Clone)]
pub struct CompositeFont {
    pub subtype: PdfName, // /Type0
    pub base_font: PdfName,
    pub encoding: Arc<CMap>,
    pub descendant_fonts: Vec<Arc<FontResource>>, // Usually contains one CIDFont
    pub to_unicode: Option<Arc<CMap>>,
}

impl FontResource {
    pub fn base_font(&self) -> &PdfName {
        match self {
            Self::Simple(f) => &f.base_font,
            Self::Composite(f) => &f.base_font,
            Self::CID(f) => &f.base_font,
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
                        if idx < f.widths.len() { return f.widths[idx]; }
                    }
                }
                f.descriptor.as_ref().map(|d| d.missing_width).unwrap_or(0.0)
            }
            Self::Composite(f) => {
                // 1. Map code to CID using CMap
                match f.encoding.lookup(code) {
                    Some(MappingResult::Cid(cid)) => {
                        // 2. Lookup width in descendant CIDFont
                        for descendant in &f.descendant_fonts {
                            if let FontResource::CID(cid_font) = descendant.as_ref() {
                                return cid_font.widths.get(cid);
                            }
                        }
                        0.0
                    }
                    _ => 0.0,
                }
            }
            Self::CID(f) => {
                // CIDFonts are not used directly for showing text, but we provide this for completeness
                let cid = if code.len() == 2 {
                    ((code[0] as u32) << 8) | (code[1] as u32)
                } else {
                    code[0] as u32
                };
                f.widths.get(cid)
            }
        }
    }

    /// Returns the vertical metrics and bounding box for the font.
    pub fn get_metrics(&self) -> FontMetrics {
        match self {
            Self::Simple(f) => f.descriptor.as_ref().map(|d| FontMetrics {
                ascent: d.ascent,
                descent: d.descent,
                cap_height: d.cap_height,
                bbox: d.font_bbox,
            }).unwrap_or(FontMetrics { ascent: 700.0, descent: -200.0, cap_height: 500.0, bbox: [0.0, 0.0, 1000.0, 1000.0] }),
            Self::Composite(f) => {
                // Return metrics of the first descendant CIDFont
                f.descendant_fonts.first().map(|d| d.get_metrics()).unwrap_or(FontMetrics { ascent: 700.0, descent: -200.0, cap_height: 500.0, bbox: [0.0, 0.0, 1000.0, 1000.0] })
            }
            Self::CID(f) => FontMetrics {
                ascent: f.descriptor.ascent,
                descent: f.descriptor.descent,
                cap_height: f.descriptor.cap_height,
                bbox: f.descriptor.font_bbox,
            },
        }
    }

    /// Maps a character code to a Unicode string.
    pub fn to_unicode(&self, code: &[u8]) -> String {
        match self {
            Self::Simple(f) => {
                // 1. Check ToUnicode CMap
                if let Some(cmap) = &f.to_unicode {
                    if let Some(MappingResult::Unicode(bytes)) = cmap.lookup(code) {
                        return decode_unicode_bytes(&bytes);
                    }
                }
                // 2. Fallback to Encoding
                if code.len() == 1 {
                    if let Some(encoding) = &f.encoding {
                         if let Some(s) = encoding.to_unicode(code[0]) {
                             return s.to_string();
                         }
                    }
                    return String::from_utf8_lossy(code).into_owned();
                }
                String::new()
            }
            Self::Composite(f) => {
                // 1. Check ToUnicode CMap
                if let Some(cmap) = &f.to_unicode {
                    if let Some(MappingResult::Unicode(bytes)) = cmap.lookup(code) {
                        return decode_unicode_bytes(&bytes);
                    }
                }
                String::new()
            }
            Self::CID(_) => String::new(),
        }
    }


    /// Recursively loads a FontResource from a PDF dictionary.
    pub fn load(dict: &BTreeMap<PdfName, Object>, resolver: &dyn Resolver) -> PdfResult<Self> {
        let subtype = dict.get(&"Subtype".into()).and_then(|o| o.as_name())
            .ok_or_else(|| PdfError::Other("Missing /Subtype in font dictionary".into()))?;

        if subtype.as_str() == "Type0" {
            Self::load_composite(dict, resolver)
        } else if subtype.as_str() == "CIDFontType0" || subtype.as_str() == "CIDFontType2" {
            Self::load_cid(dict, resolver).map(FontResource::CID)
        } else {
            Self::load_simple(dict, resolver).map(FontResource::Simple)
        }
    }

    fn load_simple(dict: &BTreeMap<PdfName, Object>, resolver: &dyn Resolver) -> PdfResult<SimpleFont> {
        let base_font = dict.get(&"BaseFont".into()).and_then(|o| o.as_name()).cloned().unwrap_or(PdfName::from("ErrorFont"));
        let first_char = dict.get(&"FirstChar".into()).and_then(|o| o.as_i64()).unwrap_or(0) as u8;
        let last_char = dict.get(&"LastChar".into()).and_then(|o| o.as_i64()).unwrap_or(0) as u8;
        let widths: Vec<f64> = dict.get(&"Widths".into()).and_then(|o| o.as_array())
            .map(|a| a.iter().filter_map(|w| w.as_f64()).collect()).unwrap_or_default();
        
        // Descriptor
        let descriptor = if let Some(d_ref) = dict.get(&"FontDescriptor".into()) {
            let d_obj = resolver.resolve_if_ref(d_ref)?;
            let d_dict = d_obj.as_dict().ok_or_else(|| PdfError::Other("Invalid /FontDescriptor".into()))?;
            Some(load_descriptor(d_dict, resolver)?)
        } else {
            None
        };

        // Encoding
        let encoding = if let Some(e_obj) = dict.get(&"Encoding".into()) {
             match e_obj {
                 Object::Name(n) => encoding::Encoding::from_name(n.as_str()),
                 _ => None, // TODO: Custom encoding with Differences
             }
        } else {
             None
        };

        // Standard 14 Fallback
        let mut final_widths = widths;
        let mut final_descriptor = descriptor;
        let mut final_encoding = encoding;

        if let Some(metrics) = standard::get_standard_metrics(base_font.as_str()) {
            if final_widths.is_empty() {
                final_widths = metrics.widths.iter().map(|&w| w as f64).collect();
            }
            if final_descriptor.is_none() {
                final_descriptor = Some(FontDescriptor {
                    font_name: base_font.clone(),
                    flags: 0,
                    font_bbox: [0.0, 0.0, 0.0, 0.0],
                    italic_angle: 0.0,
                    ascent: metrics.ascent as f64,
                    descent: metrics.descent as f64,
                    cap_height: 0.0,
                    stem_v: 0.0,
                    missing_width: 0.0,
                    font_file: None,
                });
            }
            if final_encoding.is_none() {
                final_encoding = Some(metrics.default_encoding.clone());
            }
        }

        Ok(SimpleFont {
            subtype: PdfName::from("Simple"),
            base_font,
            first_char,
            last_char,
            widths: final_widths,
            descriptor: final_descriptor,
            encoding: final_encoding,
            to_unicode: None,
        })
    }

    fn load_composite(dict: &BTreeMap<PdfName, Object>, resolver: &dyn Resolver) -> PdfResult<Self> {
        let base_font = dict.get(&"BaseFont".into()).and_then(|o| o.as_name()).cloned().unwrap_or(PdfName::from("ErrorFont"));
        
        // CMap (Encoding)
        let encoding_obj = dict.get(&"Encoding".into()).ok_or_else(|| PdfError::Other("Missing /Encoding in Type0 font".into()))?;
        let encoding = match encoding_obj {
            Object::Name(n) => {
                crate::font::cmap::get_builtin_cmap(n.as_str())
                    .ok_or_else(|| PdfError::Other(format!("Predefined CMap {} not supported yet", n.as_str())))?
            }
            _ => {
                let s = resolver.resolve_if_ref(encoding_obj)?;
                if let Object::Stream(_, data) = s {
                    Arc::new(CMap::parse(&data)?)
                } else {
                    return Err(PdfError::Other("Invalid /Encoding stream".into()));
                }
            }
        };

        // DescendantFonts
        let descendant_arr = dict.get(&"DescendantFonts".into()).and_then(|o| o.as_array())
            .ok_or_else(|| PdfError::Other("Missing /DescendantFonts in Type0 font".into()))?;
        let mut descendant_fonts = Vec::new();
        for d_ref in descendant_arr.iter() {
            let d_dict = resolver.resolve_if_ref(d_ref)?.as_dict().ok_or(PdfError::Other("Invalid descendant font".into()))?.clone();
            descendant_fonts.push(Arc::new(Self::load(&d_dict, resolver)?));
        }

        Ok(FontResource::Composite(CompositeFont {
            subtype: PdfName::from("Type0"),
            base_font,
            encoding,
            descendant_fonts,
            to_unicode: None,
        }))
    }

    fn load_cid(dict: &BTreeMap<PdfName, Object>, resolver: &dyn Resolver) -> PdfResult<CIDFont> {
        let base_font = dict.get(&"BaseFont".into()).and_then(|o| o.as_name()).cloned().unwrap_or(PdfName::from("ErrorFont"));
        let subtype = dict.get(&"Subtype".into()).and_then(|o| o.as_name()).cloned().unwrap_or(PdfName::from("CIDFontType0"));
        
        // Widths /W and /DW
        let default_width = dict.get(&"DW".into()).and_then(|o| o.as_f64()).unwrap_or(1000.0);
        let mut widths = CIDWidths { default_width, widths: BTreeMap::new() };
        if let Some(w_arr) = dict.get(&"W".into()).and_then(|o| o.as_array()) {
            let mut i = 0;
            while i < w_arr.len() {
                let first = w_arr[i].as_i64().unwrap_or(0) as u32;
                let next = &w_arr[i+1];
                if let Some(w_list) = next.as_array() {
                    for (idx, w) in w_list.iter().enumerate() {
                        widths.widths.insert(first + idx as u32, w.as_f64().unwrap_or(default_width));
                    }
                    i += 2;
                } else if let Some(last) = next.as_i64() {
                    let w = w_arr[i+2].as_f64().unwrap_or(default_width);
                    for cid in first..=(last as u32) {
                        widths.widths.insert(cid, w);
                    }
                    i += 3;
                } else {
                    break;
                }
            }
        }

        // Descriptor
        let d_ref = dict.get(&"FontDescriptor".into()).ok_or_else(|| PdfError::Other("Missing /FontDescriptor in CIDFont".into()))?;
        let descriptor = load_descriptor(resolver.resolve_if_ref(d_ref)?.as_dict().ok_or(PdfError::Other("Invalid FontDescriptor".into()))?, resolver)?;

        Ok(CIDFont {
            subtype,
            base_font,
            cid_system_info: BTreeMap::new(),
            widths,
            descriptor,
        })
    }
}

fn load_descriptor(dict: &BTreeMap<PdfName, Object>, _resolver: &dyn Resolver) -> PdfResult<FontDescriptor> {
    Ok(FontDescriptor {
        font_name: dict.get(&"FontName".into()).and_then(|o| o.as_name()).cloned().unwrap_or(PdfName::from("ErrorFont")),
        flags: dict.get(&"Flags".into()).and_then(|o| o.as_i64()).unwrap_or(0) as i32,
        font_bbox: [0.0, 0.0, 0.0, 0.0], // Simplified
        italic_angle: 0.0,
        ascent: dict.get(&"Ascent".into()).and_then(|o| o.as_f64()).unwrap_or(0.0),
        descent: dict.get(&"Descent".into()).and_then(|o| o.as_f64()).unwrap_or(0.0),
        cap_height: 0.0,
        stem_v: 0.0,
        missing_width: dict.get(&"MissingWidth".into()).and_then(|o| o.as_f64()).unwrap_or(0.0),
        font_file: None,
    })
}

fn decode_unicode_bytes(bytes: &[u8]) -> String {
    if bytes.len() >= 2 {
        // Try UTF-16BE (Standard for ToUnicode)
        let utf16: Vec<u16> = bytes.chunks_exact(2)
            .map(|c| ((c[0] as u16) << 8) | (c[1] as u16))
            .collect();
        if let Ok(s) = String::from_utf16(&utf16) {
            return s;
        }
    }
    // Fallback to UTF-8 or ASCII
    String::from_utf8_lossy(bytes).into_owned()
}
