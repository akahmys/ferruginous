//! Font and text resource management.
//! (ISO 32000-2:2020 Clause 9.2)

use crate::core::{Object, Resolver, PdfError, PdfResult};
use std::collections::BTreeMap;

use crate::cmap::{CMap, MappingResult};
use ttf_parser::OutlineBuilder;
use kurbo::{BezPath, Point, Shape};

const ARIMO_REGULAR: &[u8] = include_bytes!("../assets/fonts/Arimo-Regular.ttf");
const TINOS_REGULAR: &[u8] = include_bytes!("../assets/fonts/Tinos-Regular.ttf");
const COUSINE_REGULAR: &[u8] = include_bytes!("../assets/fonts/Cousine-Regular.ttf");

/// ISO 32000-2:2020 Clause 9.6 - Simple Fonts
/// ISO 32000-2:2020 Clause 9.7 - Composite Fonts (Type 0)
/// Represents a PDF Font (Clause 9).
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The font subtype (e.g., /Type1, /TrueType, /Type0).
    pub subtype: Vec<u8>,
    /// The base font name.
    pub base_font: Vec<u8>,
    /// The first character code defined in the widths array.
    pub first_char: i32,
    /// The last character code defined in the widths array.
    pub last_char: i32,
    /// The widths of characters from `first_char` to `last_char`.
    pub widths: Vec<f64>,
    /// The font descriptor containing additional metrics.
    pub descriptor: Option<FontDescriptor>,
    /// The encoding or `CMap` for mapping codes to glyphs/CIDs.
    pub encoding_cmap: Option<CMap>,
    /// The `ToUnicode` `CMap` for mapping codes to Unicode.
    pub to_unicode: Option<CMap>,
    /// The CID-to-width mapping for `CIDFonts`.
    pub cid_widths: BTreeMap<u32, f64>,
    /// The default width for `CIDFonts`.
    pub dw: f64,
    /// The CID-to-GID mapping (for CIDFontType2).
    pub cid_to_gid_map: Option<Vec<u16>>,
}

/// Represents a Font Descriptor (Clause 9.8).
#[derive(Debug, Clone, PartialEq)]
pub struct FontDescriptor {
    /// The PostScript name of the font.
    pub font_name: Vec<u8>,
    /// Font flags (Clause 9.8.2).
    pub flags: i32,
    /// The font bounding box.
    pub font_bbox: [f64; 4],
    /// The italic angle.
    pub italic_angle: f64,
    /// The maximal ascent.
    pub ascent: f64,
    /// The maximal descent.
    pub descent: f64,
    /// The cap height.
    pub cap_height: f64,
    /// The vertical stem width.
    pub stem_v: f64,
    /// The width to use for missing characters.
    pub missing_width: f64,
}

impl Font {
    /// Creates a Font instance from a dictionary.
    /// (ISO 32000-2:2020 Clause 9.6)
    pub fn from_dict(dict: &BTreeMap<Vec<u8>, Object>, resolver: &dyn Resolver) -> PdfResult<Self> {
        let subtype = match dict.get(b"Subtype".as_ref()) {
            Some(Object::Name(n)) => n.to_vec(),
            _ => return Err(PdfError::InvalidType { expected: "Name (/Subtype)".into(), found: "Missing or invalid".into() }),
        };

        let base_font = match dict.get(b"BaseFont".as_ref()) {
            Some(Object::Name(n)) => n.to_vec(),
            _ => Vec::new(),
        };

        let first_char = Self::get_int_param(dict, b"FirstChar", 0) as i32;
        let last_char = Self::get_int_param(dict, b"LastChar", 0) as i32;

        let widths = dict.get(b"Widths".as_ref())
            .map_or(Ok(Vec::new()), |obj| Self::parse_widths(obj, resolver))?;

        let descriptor = dict.get(b"FontDescriptor".as_ref())
            .map_or(Ok(None), |obj| FontDescriptor::from_obj(obj, resolver).map(Some))?;

        let encoding_cmap = dict.get(b"Encoding".as_ref())
            .map_or(Ok(None), |obj| Self::resolve_cmap(obj, resolver))?;

        let to_unicode = dict.get(b"ToUnicode".as_ref())
            .map_or(Ok(None), |obj| Self::resolve_cmap(obj, resolver))?;

        if subtype == b"Type0" {
            return Self::from_type0_dict(dict, subtype, base_font, encoding_cmap, to_unicode, resolver);
        }

        let (cid_widths, dw, cid_to_gid_map) = Self::parse_cid_metrics(dict, &subtype, resolver);

        Ok(Self {
            subtype, base_font, first_char, last_char, widths,
            descriptor, encoding_cmap, to_unicode, cid_widths, dw, cid_to_gid_map,
        })
    }

    fn get_int_param(dict: &BTreeMap<Vec<u8>, Object>, key: &[u8], default: i64) -> i64 {
        match dict.get(key) {
            Some(Object::Integer(i)) => *i,
            _ => default,
        }
    }

    fn parse_cid_metrics(dict: &BTreeMap<Vec<u8>, Object>, subtype: &[u8], resolver: &dyn Resolver) -> (BTreeMap<u32, f64>, f64, Option<Vec<u16>>) {
        let mut cid_widths = BTreeMap::new();
        let mut dw = 1000.0;
        let mut cid_to_gid = None;

        if subtype == b"CIDFontType0" || subtype == b"CIDFontType2" {
            if let Some(Object::Integer(d)) = dict.get(b"DW".as_slice()) {
                dw = *d as f64;
            }
            if let Some(Object::Array(w)) = dict.get(b"W".as_slice()) {
                cid_widths = Self::parse_cid_widths(w);
            }
            
            if subtype == b"CIDFontType2" {
                if let Some(obj) = dict.get(b"CIDToGIDMap".as_slice()) {
                    cid_to_gid = Self::resolve_cid_to_gid_map(obj, resolver).ok().flatten();
                }
            }
        }
        (cid_widths, dw, cid_to_gid)
    }

    fn resolve_cid_to_gid_map(obj: &Object, resolver: &dyn Resolver) -> PdfResult<Option<Vec<u16>>> {
        let actual = match obj {
            Object::Reference(r) => resolver.resolve(&r)?,
            _ => obj.clone(),
        };

        if let Object::Name(ref name) = actual {
            if name.as_slice() == b"Identity" {
                return Ok(None); // None means identity mapping
            }
        }

        if let Object::Stream(dict, data) = actual {
            let decoded = crate::filter::decode_stream(&dict, &data)?;
            let mut gids = Vec::with_capacity(decoded.len() / 2);
            for chunk in decoded.chunks_exact(2) {
                gids.push(u16::from_be_bytes([chunk[0], chunk[1]]));
            }
            return Ok(Some(gids));
        }

        Ok(None)
    }

    fn from_type0_dict(
        dict: &BTreeMap<Vec<u8>, Object>,
        subtype: Vec<u8>,
        base_font: Vec<u8>,
        encoding_cmap: Option<CMap>,
        to_unicode: Option<CMap>,
        resolver: &dyn Resolver,
    ) -> PdfResult<Self> {
        let df = dict.get(b"DescendantFonts".as_slice())
            .and_then(|o| if let Object::Array(a) = o { Some(a) } else { None })
            .ok_or_else(|| PdfError::InvalidType { expected: "Array (/DescendantFonts)".into(), found: "Missing".into() })?;
        
        let first_df = df.first().ok_or_else(|| PdfError::InvalidType { expected: "Indirect Reference".into(), found: "Empty /DescendantFonts".into() })?;
        let df_obj = match first_df {
            Object::Reference(r) => resolver.resolve(&r)?,
            _ => first_df.clone(),
        };
        
        if let Object::Dictionary(df_dict) = df_obj {
            let descendant = Self::from_dict(&df_dict, resolver)?;
            Ok(Self {
                subtype, base_font,
                first_char: descendant.first_char,
                last_char: descendant.last_char,
                widths: descendant.widths,
                descriptor: descendant.descriptor,
                encoding_cmap, to_unicode,
                cid_widths: descendant.cid_widths,
                dw: descendant.dw,
                cid_to_gid_map: descendant.cid_to_gid_map,
            })
        } else {
            Err(PdfError::InvalidType { expected: "Dictionary (DescendantFont)".into(), found: format!("{df_obj:?}") })
        }
    }

    /// Clause 9.6.1 - Returns the width of a character glyph in text space (usually units of 1/1000).
    #[must_use] pub fn glyph_width(&self, char_code: u8) -> f64 {
        let n = i32::from(char_code);
        if n >= self.first_char && n <= self.last_char {
            let index = (n - self.first_char) as usize;
            if index < self.widths.len() {
                return self.widths[index];
            }
        }
        
        // Fallback to MissingWidth from descriptor if available
        if let Some(ref desc) = self.descriptor {
            if desc.missing_width > 0.0 {
                return desc.missing_width;
            }
        }
        
        0.0 // Default to 0.0 as per Clause 9.6.1
    }

    /// Clause 9.7.4.3 - Returns the width of a CID in text space.
    #[must_use] pub fn cid_width(&self, cid: u32) -> f64 {
        debug_assert!(self.subtype.starts_with(b"CIDFont") || self.subtype == b"Type0", "cid_width: not a CIDFont or Type0");
        self.cid_widths.get(&cid).copied().unwrap_or(self.dw)
    }

    /// High-level method to get width of a character (possibly multi-byte).
    #[must_use] pub fn char_width(&self, code: &[u8]) -> f64 {
        debug_assert!(!code.is_empty(), "char_width: code empty");
        if let Some(ref cmap) = self.encoding_cmap {
            if let Some(MappingResult::Cid(cid)) = cmap.lookup(code) {
                return self.cid_width(cid);
            }
        }
        
        // Fallback to simple 1-byte lookup if CMap fails or not present
        if code.len() == 1 {
            self.glyph_width(code[0])
        } else {
            self.dw // Fallback for multi-byte with no CID mapping
        }
    }

    /// Maps a character code to a Unicode string.
    /// (ISO 32000-2:2020 Clause 9.10.2)
    #[must_use] pub fn to_unicode_string(&self, code: &[u8]) -> String {
        // 1. Try ToUnicode CMap
        if let Some(ref cmap) = self.to_unicode {
            if let Some(MappingResult::Unicode(bytes)) = cmap.lookup(code) {
                return Self::decode_utf16be(&bytes);
            }
        }

        // 2. Fallback to Encoding CMap (if it provides Unicode)
        if let Some(ref cmap) = self.encoding_cmap {
            if let Some(MappingResult::Unicode(bytes)) = cmap.lookup(code) {
                return Self::decode_utf16be(&bytes);
            }
        }

        // 3. Last resort: Treat as MacRoman or WinAnsi 1-byte (Simplified for now)
        if code.len() == 1 {
            return (code[0] as char).to_string();
        }

        String::new()
    }

    fn decode_utf16be(bytes: &[u8]) -> String {
        let mut u16_data = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks_exact(2) {
            u16_data.push(u16::from_be_bytes([chunk[0], chunk[1]]));
        }
        if u16_data.is_empty() && !bytes.is_empty() {
             // Fallback for 1-byte "Unicode" mappings sometimes found in CMap
             return String::from_utf8_lossy(bytes).into_owned();
        }
        String::from_utf16_lossy(&u16_data)
    }

    /// Returns the bounding box of a glyph in glyph space (1/1000 units).
    #[must_use] pub fn glyph_bbox(&self, code: &[u8]) -> kurbo::Rect {
        if let Ok(Some(path)) = self.get_glyph_path(code) {
             path.bounding_box()
        } else {
            let width = self.char_width(code);
            let (ascent, descent) = if let Some(ref desc) = self.descriptor {
                (desc.ascent, desc.descent)
            } else {
                (800.0, -200.0)
            };
            kurbo::Rect::new(0.0, descent, width, ascent)
        }
    }

    /// Extracts the actual path (outline) of a glyph.
    pub fn get_glyph_path(&self, code: &[u8]) -> PdfResult<Option<BezPath>> {
        // 1. Resolve GID/CID
        let gid = if let Some(ref map) = self.cid_to_gid_map {
             // CIDFontType2 with explicit map
             if let Some(MappingResult::Cid(cid)) = self.encoding_cmap.as_ref().and_then(|m| m.lookup(code)) {
                 map.get(cid as usize).copied().map(ttf_parser::GlyphId)
             } else { None }
        } else {
            // Simple font or Type 0 fallback
            if code.len() == 1 {
                Some(ttf_parser::GlyphId(code[0] as u16))
            } else { None }
        };

        let gid = match gid {
            Some(g) => g,
            None => return Ok(None),
        };

        // 2. Load font data (Simplified: using bundled Arimo for Helvetica as example)
        let font_data = match self.base_font.as_slice() {
            b"Helvetica" | b"Helvetica-Bold" | b"Arial" => ARIMO_REGULAR,
            b"Times-Roman" | b"Times-Bold" | b"TimesNewRoman" => TINOS_REGULAR,
            b"Courier" | b"Courier-Bold" => COUSINE_REGULAR,
            _ => ARIMO_REGULAR, // Default fallback
        };

        if let Ok(face) = ttf_parser::Face::parse(font_data, 0) {
            let mut builder = BezPathBuilder::new();
            if face.outline_glyph(gid, &mut builder).is_some() {
                // PDF expects 1/1000 units. TrueType is usually 2048 or something else.
                let scale = 1000.0 / face.units_per_em() as f64;
                let mut path = builder.finish();
                path.apply_affine(kurbo::Affine::scale(scale));
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    fn parse_widths(obj: &Object, resolver: &dyn Resolver) -> PdfResult<Vec<f64>> {
        let actual_obj = match obj {
            Object::Reference(r) => resolver.resolve(r)?,
            _ => obj.clone(),
        };

        if let Object::Array(arr) = actual_obj {
            Ok(arr.iter().filter_map(|o| match o {
                Object::Integer(i) => Some(*i as f64),
                Object::Real(f) => Some(*f),
                _ => None,
            }).collect())
        } else {
            Ok(Vec::new())
        }
    }

    fn resolve_cmap(obj: &Object, resolver: &dyn Resolver) -> PdfResult<Option<CMap>> {
        let actual = match obj {
            Object::Reference(r) => resolver.resolve(&r)?,
            _ => obj.clone(),
        };

        match actual {
            Object::Name(name) => {
                let name_str = String::from_utf8_lossy(&name);
                if let Some(cmap) = CMap::new_predefined(&name_str) {
                    Ok(Some(cmap))
                } else {
                    let mut cmap = CMap::new();
                    cmap.name = name_str.into_owned();
                    Ok(Some(cmap))
                }
            }
            Object::Stream(dict, data) => {
                let decoded = crate::filter::decode_stream(&dict, &data)?;
                let cmap = CMap::parse(&decoded)?;
                Ok(Some(cmap))
            }
            _ => Ok(None)
        }
    }

    fn parse_cid_widths(w_arr: &Vec<Object>) -> BTreeMap<u32, f64> {
        let mut res = BTreeMap::new();
        let mut i = 0;
        while i < w_arr.len() {
            if let Some(Object::Integer(c_start)) = w_arr.get(i) {
                let start = *c_start as u32;
                if let Some(next) = w_arr.get(i + 1) {
                    match next {
                        Object::Array(widths) => {
                            for (idx, w_obj) in widths.iter().enumerate() {
                                let w = match w_obj {
                                    Object::Integer(n) => *n as f64,
                                    Object::Real(f) => *f,
                                    _ => 0.0,
                                };
                                res.insert(start + idx as u32, w);
                            }
                            i += 2;
                        }
                        Object::Integer(c_end) => {
                            let end = *c_end as u32;
                            if let Some(w_obj) = w_arr.get(i + 2) {
                                let w = match w_obj {
                                    Object::Integer(n) => *n as f64,
                                    Object::Real(f) => *f,
                                    _ => 0.0,
                                };
                                for id in start..=end {
                                    res.insert(id, w);
                                }
                            }
                            i += 3;
                        }
                        _ => i += 1,
                    }
                } else { break; }
            } else { break; }
        }
        res
    }

    /// Returns true if this font is vertical (WMode 1).
    pub fn is_vertical(&self) -> bool {
        self.encoding_cmap.as_ref().map_or(false, |c| c.is_vertical)
    }
}

impl FontDescriptor {
    /// Creates a `FontDescriptor` instance from an object.
    /// (ISO 32000-2:2020 Clause 9.8)
    pub fn from_obj(obj: &Object, resolver: &dyn Resolver) -> PdfResult<Self> {
        let dict_obj = match obj {
            Object::Reference(r) => resolver.resolve(r)?,
            _ => obj.clone(),
        };
        let (expected, found) = ("Dictionary (FontDescriptor)", format!("{dict_obj:?}"));
        let dict = if let Object::Dictionary(d) = dict_obj { d } else { return Err(PdfError::InvalidType { expected: expected.into(), found }); };

        let font_name = match dict.get(b"FontName".as_ref()) {
            Some(Object::Name(n)) => n.to_vec(),
            _ => Vec::new(),
        };

        let flags = match dict.get(b"Flags".as_ref()) {
            Some(Object::Integer(i)) => *i as i32,
            _ => 0,
        };

        let font_bbox = match dict.get(b"FontBBox".as_ref()) {
            Some(Object::Array(a)) if a.len() == 4 => {
                let v: Vec<f64> = a.iter().filter_map(|o| match o {
                    Object::Integer(i) => Some(*i as f64),
                    Object::Real(f) => Some(*f),
                    _ => None,
                }).collect();
                if v.len() == 4 { [v[0], v[1], v[2], v[3]] } else { [0.0; 4] }
            }
            _ => [0.0; 4],
        };

        let italic_angle = Self::get_f64(&dict, b"ItalicAngle");
        let ascent = Self::get_f64(&dict, b"Ascent");
        let descent = Self::get_f64(&dict, b"Descent");
        let cap_height = Self::get_f64(&dict, b"CapHeight");
        let stem_v = Self::get_f64(&dict, b"StemV");
        let missing_width = Self::get_f64(&dict, b"MissingWidth");

        Ok(Self {
            font_name,
            flags,
            font_bbox,
            italic_angle,
            ascent,
            descent,
            cap_height,
            stem_v,
            missing_width,
        })
    }

    fn get_f64(dict: &BTreeMap<Vec<u8>, Object>, key: &[u8]) -> f64 {
        match dict.get(key) {
            Some(Object::Integer(i)) => *i as f64,
            Some(Object::Real(f)) => *f,
            _ => 0.0,
        }
    }
}

struct BezPathBuilder {
    path: BezPath,
}

impl BezPathBuilder {
    fn new() -> Self {
        Self { path: BezPath::new() }
    }
    fn finish(self) -> BezPath {
        self.path
    }
}

impl OutlineBuilder for BezPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(Point::new(x as f64, y as f64));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(Point::new(x as f64, y as f64));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(Point::new(x1 as f64, y1 as f64), Point::new(x as f64, y as f64));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.curve_to(Point::new(x1 as f64, y1 as f64), Point::new(x2 as f64, y2 as f64), Point::new(x as f64, y as f64));
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}
