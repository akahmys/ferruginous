//! PDF Font Engine (ISO 32000-2:2020 Clause 9)

pub mod cmap;

use crate::{Document, Object, PdfName, PdfResult, PdfError};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::handle::Handle;

/// Logical representation of a PDF Font (ISO 32000-2 Clause 9.2).
#[derive(Clone)]
pub struct FontResource {
    pub subtype: PdfName,
    pub base_font: PdfName,
    pub first_char: i32,
    pub last_char: i32,
    pub widths: BTreeMap<u32, f32>,
    pub vertical_widths: BTreeMap<u32, (f32, f32, f32)>, // (w1, v_x, v_y)
    pub default_width: f32,
    pub encoding: Option<cmap::CMap>,
    pub to_unicode: Option<cmap::CMap>,
    pub wmode: i32,
    pub cid_to_gid_map: Option<Vec<u16>>,
    pub data: Option<Arc<Vec<u8>>>,
    pub font_descriptor: Option<Handle<Object>>,
    pub is_legacy_distiller: bool,
    /// Unified mapping: UTF-8 String -> GID
    /// This is used during Arena Expansion for restructuring content streams.
    pub unified_map: BTreeMap<String, u32>,
}

/// Summary of a font used in the document.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FontSummary {
    /// The base name of the font.
    pub name: String,
    /// The font type (Type1, TrueType, Type0, etc.).
    pub font_type: String,
    /// Whether the font is embedded in the PDF.
    pub is_embedded: bool,
    /// Whether the font is a subset of the original font.
    pub is_subset: bool,
    /// The character encoding used by the font.
    pub encoding: String,
}

impl FontResource {
    /// Loads a Font resource from a PDF dictionary.
    pub fn load(dict: &BTreeMap<Handle<PdfName>, Object>, doc: &Document) -> PdfResult<Self> {
        let arena = doc.arena();
        
        // 1. More robust subtype and name detection
        let subtype_obj = dict.get(&arena.name("Subtype")).map(|o| o.resolve(arena));
        let subtype_name = subtype_obj.as_ref().and_then(|o| o.as_name()).and_then(|h| arena.get_name(h));
        let _subtype_str = subtype_name.as_ref().map(|n| n.as_str()).unwrap_or("Unknown");
        
        let base_font_obj = dict.get(&arena.name("BaseFont")).map(|o| o.resolve(arena));
        let base_font_name = base_font_obj.as_ref().and_then(|o| o.as_name()).and_then(|h| arena.get_name(h));
        let base_font_str = base_font_name.as_ref().map(|n| n.as_str().to_string()).unwrap_or_else(|| "Untitled".to_string());



        let subtype = subtype_name.ok_or_else(|| PdfError::Other(format!("Missing font subtype for {}", base_font_str)))?;
        let base_font = base_font_name.unwrap_or_else(|| PdfName::new("Untitled"));
        
        let mut font_data = None;
        let mut cid_to_gid_map = None;
        if let Some(fd_obj) = dict.get(&arena.name("FontDescriptor")) {
            font_data = Self::extract_font_data(fd_obj, doc);
        }

        let mut to_unicode = None;
        if let Some(tu_obj) = dict.get(&arena.name("ToUnicode"))
            && let Ok(data) = doc.decode_stream(&tu_obj.resolve(arena))
                && let Ok(m) = cmap::CMap::parse(&data) {
                    to_unicode = Some(m);
                }

        let mut is_legacy_distiller = false;
        if let Some(tu) = &to_unicode {
            // Detect legacy distiller by checking if ASCII range is repurposed
            for (code_vec, uni_str) in &tu.mappings {
                if code_vec.len() == 1 && code_vec[0] >= 0x20 && code_vec[0] <= 0x7E
                    && uni_str.chars().any(|c| (c as u32) > 0xFF) {
                        is_legacy_distiller = true;
                        break;
                    }
            }
        }

        let mut encoding = None;
        if let Some(enc_obj) = dict.get(&arena.name("Encoding")) {
            let enc = enc_obj.resolve(arena);
            match enc {
                Object::Name(h) => {
                    if let Some(name) = arena.get_name(h) {
                        let name_str = name.as_str();
                        encoding = cmap::CMap::load_named(name_str);
                        if encoding.is_none() {
                            match name_str {
                                "Identity-H" => encoding = Some(cmap::CMap::identity_h()),
                                "Identity-V" => encoding = Some(cmap::CMap::identity_v()),
                                "90ms-RKSJ-H" => encoding = Some(cmap::CMap::rksj_h()),
                                "UniJIS-UTF16-H" => encoding = Some(cmap::CMap::unijis_h()),
                                _ => {}
                            }
                        }
                    }
                }
                Object::Stream(_, _) => {
                    if let Ok(data) = doc.decode_stream(&enc)
                        && let Ok(m) = cmap::CMap::parse(&data) {
                            encoding = Some(m);
                        }
                }
                Object::Dictionary(h) => {
                    if let Some(enc_dict) = arena.get_dict(h) {
                        let mut cmap = cmap::CMap::default();
                        
                        // 1. Handle BaseEncoding
                        if let Some(base_obj) = enc_dict.get(&arena.name("BaseEncoding"))
                            && let Some(base_name) = base_obj.resolve(arena).as_name().and_then(|h| arena.get_name(h))
                                && let Some(base_cmap) = cmap::CMap::load_named(base_name.as_str()) {
                                    cmap = base_cmap;
                                }
                        
                        // 2. Handle Differences
                        if let Some(diff_obj) = enc_dict.get(&arena.name("Differences"))
                            && let Object::Array(ah) = diff_obj.resolve(arena)
                                && let Some(arr) = arena.get_array(ah) {
                                    let mut current_code = 0u32;
                                    for item in arr {
                                        match item.resolve(arena) {
                                            crate::object::Object::Integer(code) => {
                                                current_code = code as u32;
                                            }
                                            crate::object::Object::Name(name_h) => {
                                                if let Some(glyph_name) = arena.get_name(name_h) {
                                                    let code_vec = vec![current_code as u8];
                                                    cmap.mappings.insert(code_vec, format!("/{}", glyph_name.as_str()));
                                                    current_code += 1;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                        encoding = Some(cmap);
                    }
                }
                _ => {}
            }
        }

        let mut widths = BTreeMap::new();
        let mut v_widths = BTreeMap::new();
        let mut default_width = 1000.0;
        let mut first = 0;
        let mut last = 0;
        let mut font_descriptor = None;
        if let Some(fd_obj) = dict.get(&arena.name("FontDescriptor")) {
            font_descriptor = fd_obj.as_reference();
        }

        // For Type0 fonts, look into DescendantFonts
        if subtype.as_str() == "Type0" {
            if let Some(df_obj) = dict.get(&arena.name("DescendantFonts"))
                && let Object::Array(ah) = df_obj.resolve(arena)
                    && let Some(arr) = arena.get_array(ah)
                        && let Some(df_dict_obj) = arr.first()
                            && let Object::Dictionary(dfh) = df_dict_obj.resolve(arena)
                                && let Some(df_dict) = arena.get_dict(dfh) {
                                    // Extract font data and descriptor from descendant if not already found
                                    if font_data.is_none()
                                        && let Some(fd_obj) = df_dict.get(&arena.name("FontDescriptor")) {
                                            font_data = Self::extract_font_data(fd_obj, doc);
                                            if font_descriptor.is_none() {
                                                font_descriptor = fd_obj.as_reference();
                                            }
                                        }

                                    // Parse CIDToGIDMap
                                    if let Some(map_obj) = df_dict.get(&arena.name("CIDToGIDMap")) {
                                        let resolved = map_obj.resolve(arena);
                                        if let Some(name) = resolved.as_name().and_then(|h| arena.get_name(h)) {
                                            if name.as_str() == "Identity" {
                                                // Identity mapping (CID == GID)
                                            }
                                        } else if let Ok(data) = doc.decode_stream(&resolved) {
                                            let mut map = Vec::with_capacity(data.len() / 2);
                                            for chunk in data.chunks_exact(2) {
                                                map.push(u16::from_be_bytes([chunk[0], chunk[1]]));
                                            }
                                            cid_to_gid_map = Some(map);
                                        }
                                    }
                                    // Parse W (Horizontal widths)
                                    if let Some(w_obj) = df_dict.get(&arena.name("W"))
                                        && let Object::Array(wah) = w_obj.resolve(arena)
                                            && let Some(w_arr) = arena.get_array(wah) {
                                                let mut i = 0;
                                                while i < w_arr.len() {
                                                    let first_cid = w_arr[i].resolve(arena).as_integer().unwrap_or(0) as u32;
                                                    let next_obj = w_arr[i+1].resolve(arena);
                                                    if let Object::Array(iah) = next_obj {
                                                        if let Some(i_arr) = arena.get_array(iah) {
                                                            for (idx, w) in i_arr.iter().enumerate() {
                                                                widths.insert(first_cid + idx as u32, w.resolve(arena).as_f64().unwrap_or(1000.0) as f32);
                                                            }
                                                        }
                                                        i += 2;
                                                    } else {
                                                        let last_cid = next_obj.as_integer().unwrap_or(0) as u32;
                                                        let w = w_arr[i+2].resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                                                        for cid in first_cid..=last_cid {
                                                            widths.insert(cid, w);
                                                        }
                                                        i += 3;
                                                    }
                                                }
                                            }
                                    // Parse DW (Default Width)
                                    if let Some(dw_obj) = df_dict.get(&arena.name("DW")) {
                                        default_width = dw_obj.resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                                    }

                                    // Parse W2 (Vertical widths and vectors)
                                    if let Some(w2_obj) = df_dict.get(&arena.name("W2"))
                                        && let Object::Array(wah) = w2_obj.resolve(arena)
                                            && let Some(w2_arr) = arena.get_array(wah) {
                                                let mut i = 0;
                                                while i < w2_arr.len() {
                                                    let first_cid = w2_arr[i].resolve(arena).as_integer().unwrap_or(0) as u32;
                                                    let next_obj = w2_arr[i+1].resolve(arena);
                                                    if let Object::Array(iah) = next_obj {
                                                        if let Some(i_arr) = arena.get_array(iah) {
                                                            for (idx, chunk) in i_arr.chunks_exact(3).enumerate() {
                                                                let w1_y = chunk[0].resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                                                                let v_x = chunk[1].resolve(arena).as_f64().unwrap_or(default_width as f64 / 2.0) as f32;
                                                                let v_y = chunk[2].resolve(arena).as_f64().unwrap_or(880.0) as f32;
                                                                v_widths.insert(first_cid + idx as u32, (w1_y, v_x, v_y));
                                                            }
                                                        }
                                                        i += 2;
                                                    } else {
                                                        let last_cid = next_obj.as_integer().unwrap_or(0) as u32;
                                                        let w1_y = w2_arr[i+2].resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                                                        let v_x = w2_arr[i+3].resolve(arena).as_f64().unwrap_or(default_width as f64 / 2.0) as f32;
                                                        let v_y = w2_arr[i+4].resolve(arena).as_f64().unwrap_or(880.0) as f32;
                                                        for cid in first_cid..=last_cid {
                                                            v_widths.insert(cid, (w1_y, v_x, v_y));
                                                        }
                                                        i += 5;
                                                    }
                                                }
                                            }
                                }
        } else {
            // Standard font widths (FirstChar, LastChar, Widths)
            if let Some(first_obj) = dict.get(&arena.name("FirstChar")) {
                first = first_obj.resolve(arena).as_integer().unwrap_or(0) as i32;
            }
            if let Some(last_obj) = dict.get(&arena.name("LastChar")) {
                last = last_obj.resolve(arena).as_integer().unwrap_or(0) as i32;
            }
            if let Some(widths_obj) = dict.get(&arena.name("Widths"))
                && let Object::Array(ah) = widths_obj.resolve(arena)
                    && let Some(arr) = arena.get_array(ah) {
                        for (idx, w) in arr.iter().enumerate() {
                            widths.insert((first + idx as i32) as u32, w.resolve(arena).as_f64().unwrap_or(0.0) as f32);
                        }
                    }
        }

        let mut wmode = 0;
        if let Some(enc_obj) = dict.get(&arena.name("Encoding")) {
            let enc = enc_obj.resolve(arena);
            if let Some(name_h) = enc.as_name()
                && let Some(name) = arena.get_name(name_h)
                    && (name.as_str().ends_with("-V") || name.as_str() == "V") {
                        wmode = 1;
                    }
        }

        let mut resource = Self {
            subtype,
            base_font,
            first_char: first,
            last_char: last,
            widths,
            vertical_widths: v_widths,
            default_width,
            encoding,
            to_unicode,
            wmode,
            cid_to_gid_map,
            data: font_data.map(Arc::new),
            font_descriptor,
            is_legacy_distiller,
            unified_map: BTreeMap::new(),
        };



        resource.build_unified_map();
        Ok(resource)
    }

    pub fn has_any_mapping(&self) -> bool {
        (self.to_unicode.as_ref().map(|m| !m.mappings.is_empty()).unwrap_or(false)) || 
        (self.encoding.as_ref().map(|m| !m.mappings.is_empty()).unwrap_or(false))
    }

    /// Loads a Font resource directly from lopdf objects (used during ingest refinement).
    pub fn from_lopdf(
        _id: (u32, u16),
        dict: &lopdf::Dictionary,
        doc: &lopdf::Document,
    ) -> PdfResult<Self> {
        let subtype = dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok())
            .map(|n| PdfName::new(&String::from_utf8_lossy(n)))
            .ok_or_else(|| PdfError::Other("Missing font subtype".into()))?;

        let base_font = dict.get(b"BaseFont").ok().and_then(|o| o.as_name().ok())
            .map(|n| PdfName::new(&String::from_utf8_lossy(n)))
            .unwrap_or(PdfName::new("Unknown"));

        let mut to_unicode = None;
        if let Ok(to_uni_obj) = dict.get(b"ToUnicode")
            && let Ok(stream) = doc.get_object(to_uni_obj.as_reference()?).and_then(|o| o.as_stream())
                && let Ok(m) = cmap::CMap::parse(&stream.content) {
                    to_unicode = Some(m);
                }

        let mut encoding = None;
        if let Ok(enc_obj) = dict.get(b"Encoding") {
            match enc_obj {
                lopdf::Object::Name(n) => {
                    let name_str = String::from_utf8_lossy(n);
                    encoding = cmap::CMap::load_named(&name_str);
                    if encoding.is_none() {
                        match name_str.as_ref() {
                            "Identity-H" => encoding = Some(cmap::CMap::identity_h()),
                            "Identity-V" => encoding = Some(cmap::CMap::identity_v()),
                            "90ms-RKSJ-H" => encoding = Some(cmap::CMap::rksj_h()),
                            "UniJIS-UTF16-H" => encoding = Some(cmap::CMap::unijis_h()),
                            _ => {}
                        }
                    }
                }
                lopdf::Object::Reference(rid) => {
                    if let Ok(stream) = doc.get_object(*rid).and_then(|o| o.as_stream())
                        && let Ok(m) = cmap::CMap::parse(&stream.content) {
                            encoding = Some(m);
                        }
                }
                _ => {}
            }
        }

        let mut wmode = 0;
        if let Some(enc_name) = encoding.as_ref().map(|e| e.name())
            && (enc_name.ends_with("-V") || enc_name == "V") {
                wmode = 1;
            }

        let mut resource = Self {
            subtype,
            base_font,
            first_char: 0,
            last_char: 0,
            widths: BTreeMap::new(),
            vertical_widths: BTreeMap::new(),
            default_width: 1000.0,
            encoding,
            to_unicode,
            wmode,
            cid_to_gid_map: None,
            data: None,
            font_descriptor: None,
            is_legacy_distiller: false,
            unified_map: BTreeMap::new(),
        };

        resource.build_unified_map();
        Ok(resource)
    }

    /// Builds a deterministic mapping from Unicode strings to GIDs.
    /// In case of collisions (multiple byte codes mapping to the same Unicode),
    /// it ensures unique identification by using Private Use Area (PUA) codes.
    fn build_unified_map(&mut self) {
        let mut map = BTreeMap::new();
        
        // Use the encoding map as the primary source of characters described in the PDF
        if let Some(ref enc) = self.encoding {
            // Since we can't easily iterate an opaque CMap, we look at common characters 
            // and the ToUnicode mapping if it exists.
            // For Identity-H/V fonts, we map CIDs directly to Unicode or PUA.
            
            // Hardening: For Identity-H/V, the CID is usually the Unicode or at least
            // 1:1 with it.
            if enc.name().contains("Identity") {
                // For Identity mapping, we trust the ToUnicode to tell us what these CIDs mean.
                // If ToUnicode is missing, we map CID as Unicode (Identity).
                for cid in 0..65535 {
                    let mut unicode = None;
                    if let Some(ref to_uni) = self.to_unicode {
                        let cid_bytes = if cid > 255 {
                           vec![(cid >> 8) as u8, (cid & 0xFF) as u8]
                        } else {
                           vec![cid as u8]
                        };
                        unicode = to_uni.map(&cid_bytes);
                    }
                    
                    let final_unicode = unicode.or_else(|| {
                        std::char::from_u32(cid as u32).map(|c| c.to_string())
                    });

                    if let Some(u) = final_unicode {
                        map.insert(u, cid as u32);
                    }
                }
            } else {
                // For non-Identity fonts (legacy or named CMaps), we must resolve
                // Byte -> Unicode and Byte -> CID.
                // We'll iterate through 0..255 for now (simple fonts) or handle
                // specific CMap ranges if needed.
                for b in 0..=255u8 {
                    let bytes = [b];
                    let cid = enc.to_cid(&bytes);
                    if cid > 0 {
                        let unicode = if let Some(ref to_uni) = self.to_unicode {
                            to_uni.map(&bytes)
                        } else {
                            enc.map(&bytes)
                        };

                        if let Some(u) = unicode {
                            map.insert(u, cid);
                        }
                    }
                }
            }
        }
        
        self.unified_map = map;
    }

    fn extract_font_data(fd_obj: &Object, doc: &Document) -> Option<Vec<u8>> {
        let arena = doc.arena();
        let fd_resolved = fd_obj.resolve(arena);
        if let Object::Dictionary(fdh) = fd_resolved
            && let Some(fd_dict) = arena.get_dict(fdh) {
                // Try FontFile3 (CFF, OpenType, etc.)
                if let Some(ff3) = fd_dict.get(&arena.name("FontFile3"))
                    && let Ok(data) = doc.decode_stream(&ff3.resolve(arena)) {
                        return Some(data.to_vec());
                    }
                // Try FontFile2 (TrueType)
                if let Some(ff2) = fd_dict.get(&arena.name("FontFile2"))
                    && let Ok(data) = doc.decode_stream(&ff2.resolve(arena)) {
                        return Some(data.to_vec());
                    }
                // Try FontFile (Type 1)
                if let Some(ff) = fd_dict.get(&arena.name("FontFile"))
                    && let Ok(data) = doc.decode_stream(&ff.resolve(arena)) {
                        return Some(data.to_vec());
                    }
            }
        None
    }

    pub fn glyph_width(&self, code: &[u8]) -> f32 {
        if code.is_empty() { return 0.0; }
        let cid = self.to_cid(code);
        
        if self.wmode == 1 {
            if let Some((w1_y, _, _)) = self.vertical_widths.get(&cid) {
                return *w1_y;
            }
            return 1000.0; // Default vertical advance
        }

        *self.widths.get(&cid).unwrap_or(&self.default_width)
    }

    pub fn glyph_vertical_metrics(&self, cid: u32) -> (f32, f32, f32) {
        if let Some(&metrics) = self.vertical_widths.get(&cid) {
            return metrics;
        }
        // Default values: (w1_y, v_x, v_y)
        // From PDF spec: Default v = (w0/2, 880)
        let w0 = *self.widths.get(&cid).unwrap_or(&self.default_width);
        (1000.0, w0 / 2.0, 880.0)
    }

    pub fn to_unicode(&self, code: &[u8]) -> Option<String> {
        if let Some(ref map) = self.to_unicode {
            let res = map.map(code);
            if res.is_some() { return res; }
        }

        let cid = self.to_cid(code);
        
        // System-wide fallback for Adobe-Japan1 collections
        if let Some(ucs2_map) = cmap::CMap::load_named("Adobe-Japan1-UCS2") {
            let cid_bytes = vec![(cid >> 8) as u8, (cid & 0xFF) as u8];
            if let Some(s) = ucs2_map.map(&cid_bytes) {
                return Some(s);
            }
        }
        
        // Final fallback: if CID is in ASCII range, try to interpret it as a character
        if cid < 128 && cid > 0 {
            return Some((cid as u8 as char).to_string());
        }

        // Positional preservation: always return a character to keep the glyph stream in sync
        Some("\u{FFFD}".to_string())
    }

    pub fn wmode(&self) -> i32 {
        self.wmode
    }

    pub fn to_cid(&self, code: &[u8]) -> u32 {
        if let Some(ref enc) = self.encoding {
            return enc.to_cid(code);
        }
        // Fallback for simple fonts
        if code.len() == 2 {
            return ((code[0] as u32) << 8) | (code[1] as u32);
        }
        code.first().copied().unwrap_or(0) as u32
    }

    pub fn decode_next(&self, data: &[u8]) -> (usize, Option<String>) {
        if data.is_empty() { return (0, None); }

        let subtype = self.subtype.as_str();
        let is_multibyte = subtype == "Type0" || subtype == "CIDFontType0" || subtype == "CIDFontType2";
        let is_identity = self.encoding.as_ref().map(|e| e.name.contains("Identity")).unwrap_or(false);
        let min_len = if is_multibyte || is_identity { Some(2) } else { None };

        // 1. Try ToUnicode Map strictly
        if let Some(tu) = &self.to_unicode
            && let Some((len, u)) = tu.decode_next_with_min_len(data, min_len) {
                if let Some(u_str) = u {
                    return (len, Some(u_str));
                }
                return (len, None);
            }

        // 2. Try Encoding Map strictly
        if let Some(enc) = &self.encoding
            && let Some((len, u)) = enc.decode_next_with_min_len(data, min_len) {
                if let Some(u_str) = u {
                    if u_str.starts_with('/') {
                         return (len, Some(cmap::glyph_name_to_unicode(u_str.as_bytes())));
                    }
                    return (len, Some(u_str));
                }
                return (len, None);
            }

        // 3. Fallback to Heuristic consumption length
        let subtype = self.subtype.as_str();
        let is_multibyte = subtype == "Type0" || subtype == "CIDFontType0" || subtype == "CIDFontType2";
        let is_identity = self.encoding.as_ref().map(|e| e.name.contains("Identity")).unwrap_or(false);

        // Hardening: Suppress heuristic if font is known to repurpose ASCII codes (Legacy Distiller)
        if !is_multibyte && !is_identity && !self.is_legacy_distiller
            && !data.is_empty() {
                let code = data[0];
                if code > 31 && code < 127 {
                    return (1, Some(String::from_utf8_lossy(&[code]).to_string()));
                }
            }

        let consumed = if is_multibyte || is_identity { 2 } else { 1 };
        
        if data.len() < consumed {
            return (data.len(), None);
        }
        let code_bytes = &data[..consumed];

        // 4. Heuristic fallback for printable ASCII (only for simple fonts with NO reliable maps)
        let subtype_str = self.subtype.as_str();
        let is_simple = subtype_str == "Type1" || subtype_str == "TrueType" || subtype_str == "Type3";
        let has_reliable_map = self.to_unicode.is_some() || self.encoding.is_some();
        
        if is_simple && !self.is_legacy_distiller && !has_reliable_map && consumed == 1 && code_bytes[0] >= 0x20 && code_bytes[0] <= 0x7E {
            return (consumed, Some((code_bytes[0] as char).to_string()));
        }

        // 5. Hard fallback for Identity-H/V
        if is_identity && consumed == 2 {
            let val = ((code_bytes[0] as u32) << 8) | (code_bytes[1] as u32);
            if let Some(c) = std::char::from_u32(val) {
                return (consumed, Some(c.to_string()));
            }
        }

        (consumed, None)
    }
}

pub fn list_fonts(doc: &Document) -> Vec<FontSummary> {
    let arena = doc.arena();
    let mut fonts = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let type_key = arena.get_name_by_str("Type");
    let font_val = arena.get_name_by_str("Font");

    if let (Some(tk), Some(fv)) = (type_key, font_val) {
        let base_font_key = arena.get_name_by_str("BaseFont");
        let subtype_key = arena.get_name_by_str("Subtype");
        let encoding_key = arena.get_name_by_str("Encoding");
        let desc_key = arena.get_name_by_str("FontDescriptor");
        let f1_key = arena.get_name_by_str("FontFile");
        let f2_key = arena.get_name_by_str("FontFile2");
        let f3_key = arena.get_name_by_str("FontFile3");

        for handle in arena.all_dict_handles() {
            if let Some(dict) = arena.get_dict(handle)
                && dict.get(&tk).and_then(|o| o.resolve(arena).as_name()) == Some(fv) {
                    if seen.contains(&handle) { continue; }
                    seen.insert(handle);

                    let name = dict.get(&base_font_key.unwrap_or(fv))
                        .and_then(|o| o.resolve(arena).as_name())
                        .and_then(|n| arena.get_name_str(n)).unwrap_or_else(|| format!("Unnamed-<{handle:?}>"));

                    let font_type = dict.get(&subtype_key.unwrap_or(fv))
                        .and_then(|o| o.resolve(arena).as_name())
                        .and_then(|n| arena.get_name_str(n)).unwrap_or_else(|| "Type1".to_string());

                    let encoding_obj = dict.get(&encoding_key.unwrap_or(fv)).map(|o| o.resolve(arena));
                    let encoding = match encoding_obj {
                        Some(Object::Name(h)) => arena.get_name_str(h).unwrap_or_else(|| "CustomName".to_string()),
                        Some(Object::Dictionary(_)) => "CustomDict".to_string(),
                        Some(Object::Stream(_, _)) => "CustomStream".to_string(),
                        _ => "Standard".to_string(),
                    };

                    let mut is_embedded = false;
                    if let Some(desc_handle) = dict.get(&desc_key.unwrap_or(fv)).and_then(|o| o.resolve(arena).as_dict_handle())
                        && let Some(desc_dict) = arena.get_dict(desc_handle)
                        && [f1_key, f2_key, f3_key].iter().flatten().any(|k| desc_dict.contains_key(k)) {
                            is_embedded = true;
                    }

                    let is_subset = name.len() > 7 && name.as_bytes().get(6).copied() == Some(b'+');

                    fonts.push(FontSummary {
                        name,
                        font_type,
                        is_embedded,
                        is_subset,
                        encoding,
                    });
            }
        }
    }

    fonts
}
