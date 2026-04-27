//! PDF Font Engine (ISO 32000-2:2020 Clause 9)

pub mod agl;
pub mod cmap;
pub mod subset;
pub mod schema;

use crate::{Document, Object, PdfArena, PdfError, PdfName, PdfResult};
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
    /// System-wide Adobe-Japan1-UCS2 mapping for Japanese fonts.
    pub adj1_mapping: Option<cmap::CMap>,
    pub reverse_adj1_mapping: Option<BTreeMap<String, u32>>,
    /// Mapping discovered during content stream scanning (Original Bytes -> Unicode)
    pub discovered_mappings: Arc<std::sync::Mutex<BTreeMap<Vec<u8>, String>>>,
    /// Mapping from Unicode characters to internal Glyph IDs (GIDs) from the font file
    pub unicode_to_gid: BTreeMap<char, u32>,
}

#[derive(Default)]
struct FontMetrics {
    first: i32,
    last: i32,
    widths: BTreeMap<u32, f32>,
    v_widths: BTreeMap<u32, (f32, f32, f32)>,
    default_width: f32,
}

#[derive(Default)]
struct DescendantResult {
    base_font: Option<PdfName>,
    font_data: Option<Vec<u8>>,
    font_descriptor: Option<Handle<Object>>,
    cid_to_gid_map: Option<Vec<u16>>,
    metrics: FontMetrics,
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
        let subtype = Self::extract_subtype(dict, arena)?;
        let mut base_font = Self::extract_base_font(dict, arena);

        let mut font_data = None;
        if let Some(fd_obj) = dict.get(&arena.name("FontDescriptor")) {
            font_data = Self::extract_font_data(fd_obj, doc);
        }

        let to_unicode = Self::parse_to_unicode(dict, doc);
        let is_legacy_distiller = Self::detect_legacy_distiller(&to_unicode);
        let encoding = Self::parse_encoding(dict, doc);

        let mut metrics = FontMetrics::default();
        let mut font_descriptor = dict.get(&arena.name("FontDescriptor")).and_then(|o| o.as_reference());
        let mut cid_to_gid_map = None;

        if subtype.as_str() == "Type0" {
            if let Some(res) = Self::parse_descendant_font(dict, doc) {
                if base_font.as_str() == "Untitled"
                    && let Some(bf) = res.base_font { base_font = bf; }
                if font_data.is_none() { font_data = res.font_data; }
                if font_descriptor.is_none() { font_descriptor = res.font_descriptor; }
                cid_to_gid_map = res.cid_to_gid_map;
                metrics = res.metrics;
            }
        } else {
            metrics = Self::parse_standard_widths(dict, arena);
        }

        let wmode = Self::detect_wmode(dict, arena);

        let mut resource = Self {
            subtype,
            base_font,
            first_char: metrics.first,
            last_char: metrics.last,
            widths: metrics.widths,
            vertical_widths: metrics.v_widths,
            default_width: metrics.default_width,
            encoding,
            to_unicode,
            wmode,
            cid_to_gid_map,
            data: font_data.map(Arc::new),
            font_descriptor,
            is_legacy_distiller,
            unified_map: BTreeMap::new(),
            adj1_mapping: None,
            reverse_adj1_mapping: None,
            discovered_mappings: std::sync::Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::new())),
            unicode_to_gid: std::collections::BTreeMap::new(),
        };

        resource.init_adj1_mapping();
        resource.build_unified_map();
        
        // HARDENING: Populate unicode_to_gid from embedded font data if available
        if let Some(ref arc_data) = resource.data
            && let Ok(face) = ttf_parser::Face::parse(arc_data, 0) {
            let mut u2g = std::collections::BTreeMap::new();
            for table in face.tables().cmap.iter().flat_map(|t| t.subtables) {
                if table.is_unicode() {
                    table.codepoints(|cp| {
                        if let Some(c) = std::char::from_u32(cp)
                            && let Some(gid) = table.glyph_index(cp) {
                            u2g.insert(c, gid.0 as u32);
                        }
                    });
                }
            }
            resource.unicode_to_gid = u2g;
        }

        Ok(resource)
    }

    fn extract_subtype(dict: &BTreeMap<Handle<PdfName>, Object>, arena: &PdfArena) -> PdfResult<PdfName> {
        let subtype_name = dict.get(&arena.name("Subtype"))
            .and_then(|o| o.resolve(arena).as_name())
            .and_then(|h| arena.get_name(h));
        
        subtype_name.ok_or_else(|| PdfError::Other("Missing font subtype".into()))
    }

    fn extract_base_font(dict: &BTreeMap<Handle<PdfName>, Object>, arena: &PdfArena) -> PdfName {
        dict.get(&arena.name("BaseFont"))
            .and_then(|o| o.resolve(arena).as_name())
            .and_then(|h| arena.get_name(h))
            .unwrap_or_else(|| PdfName::new("Untitled"))
    }

    fn parse_to_unicode(dict: &BTreeMap<Handle<PdfName>, Object>, doc: &Document) -> Option<cmap::CMap> {
        let arena = doc.arena();
        dict.get(&arena.name("ToUnicode"))
            .and_then(|tu_obj| doc.decode_stream(&tu_obj.resolve(arena)).ok())
            .and_then(|data| cmap::CMap::parse(&data).ok())
    }

    fn detect_legacy_distiller(to_unicode: &Option<cmap::CMap>) -> bool {
        let Some(tu) = to_unicode else { return false };
        tu.mappings.iter().any(|(code_vec, uni_str)| {
            code_vec.len() == 1 && code_vec[0] >= 0x20 && code_vec[0] <= 0x7E && uni_str.chars().any(|c| (c as u32) > 0xFF)
        })
    }

    fn parse_encoding(dict: &BTreeMap<Handle<PdfName>, Object>, doc: &Document) -> Option<cmap::CMap> {
        let arena = doc.arena();
        let enc_obj = dict.get(&arena.name("Encoding"))?;
        let enc = enc_obj.resolve(arena);
        match enc {
            Object::Name(h) => {
                let name = arena.get_name(h)?;
                let name_str = name.as_str();
                cmap::CMap::load_named(name_str).or_else(|| {
                    match name_str {
                        "Identity-H" => Some(cmap::CMap::identity_h()),
                        "Identity-V" => Some(cmap::CMap::identity_v()),
                        "90ms-RKSJ-H" => Some(cmap::CMap::rksj_h()),
                        "UniJIS-UTF16-H" => Some(cmap::CMap::unijis_h()),
                        _ => None
                    }
                })
            }
            Object::Stream(_, _) => doc.decode_stream(&enc).ok().and_then(|data| cmap::CMap::parse(&data).ok()),
            Object::Dictionary(h) => Self::parse_encoding_dict(h, arena),
            _ => None,
        }
    }

    fn parse_encoding_dict(h: Handle<BTreeMap<Handle<PdfName>, Object>>, arena: &PdfArena) -> Option<cmap::CMap> {
        let enc_dict = arena.get_dict(h)?;
        let mut cmap = cmap::CMap::default();

        if let Some(base_name) = enc_dict.get(&arena.name("BaseEncoding"))
            .and_then(|o: &Object| o.resolve(arena).as_name())
            .and_then(|h: Handle<PdfName>| arena.get_name(h)) 
        {
            let name_str: String = base_name.as_str().to_string();
            if let Some(base_cmap) = cmap::CMap::load_named(&name_str) {
                cmap = base_cmap;
            }
        }

        if let Some(Object::Array(ah)) = enc_dict.get(&arena.name("Differences")).map(|o: &Object| o.resolve(arena))
            && let Some(arr) = arena.get_array(ah) {
            let mut new_mappings = (*cmap.mappings).clone();
            let mut current_code = 0u32;
            for item in arr {
                let resolved: Object = item.resolve(arena);
                match resolved {
                    Object::Integer(code) => current_code = code as u32,
                    Object::Name(name_h) => {
                        if let Some(glyph_name) = arena.get_name(name_h) {
                            let glyph_name_str: String = glyph_name.as_str().to_string();
                            new_mappings.insert(vec![current_code as u8], format!("/{}", glyph_name_str));
                            current_code += 1;
                        }
                    }
                    _ => {}
                }
            }
            cmap.mappings = Arc::new(new_mappings);
        }
        Some(cmap)
    }

    fn parse_descendant_font(dict: &BTreeMap<Handle<PdfName>, Object>, doc: &Document) -> Option<DescendantResult> {
        let arena = doc.arena();
        let df_dict_obj = dict.get(&arena.name("DescendantFonts"))?.resolve(arena).as_array().and_then(|ah| arena.get_array(ah))?.first()?.clone();
        let dfh = df_dict_obj.resolve(arena).as_dict_handle()?;
        let df_dict = arena.get_dict(dfh)?;

        let res = DescendantResult {
            base_font: df_dict.get(&arena.name("BaseFont"))
                .and_then(|o| o.resolve(arena).as_name())
                .and_then(|h| arena.get_name(h)),
            font_data: if let Some(fd_obj) = df_dict.get(&arena.name("FontDescriptor")) {
                Self::extract_font_data(fd_obj, doc)
            } else {
                None
            },
            font_descriptor: if let Some(fd_obj) = df_dict.get(&arena.name("FontDescriptor")) {
                fd_obj.as_reference()
            } else {
                None
            },
            cid_to_gid_map: Self::parse_cid_to_gid_map(&df_dict, doc),
            metrics: Self::parse_cid_metrics(&df_dict, arena),
        };

        Some(res)
    }

    fn parse_cid_to_gid_map(df_dict: &BTreeMap<Handle<PdfName>, Object>, doc: &Document) -> Option<Vec<u16>> {
        let arena = doc.arena();
        let map_obj = df_dict.get(&arena.name("CIDToGIDMap"))?;
        let resolved = map_obj.resolve(arena);
        if let Some(name) = resolved.as_name().and_then(|h| arena.get_name(h))
            && name.as_str() == "Identity" { return None; }
        let data = doc.decode_stream(&resolved).ok()?;
        Some(data.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect())
    }

    fn parse_cid_metrics(df_dict: &BTreeMap<Handle<PdfName>, Object>, arena: &PdfArena) -> FontMetrics {
        let mut metrics = FontMetrics::default();
        if let Some(dw_obj) = df_dict.get(&arena.name("DW")) {
            metrics.default_width = dw_obj.resolve(arena).as_f64().unwrap_or(1000.0) as f32;
        }
        
        if let Some(Object::Array(wah)) = df_dict.get(&arena.name("W")).map(|o: &Object| o.resolve(arena))
            && let Some(w_arr) = arena.get_array(wah) {
            let mut i: usize = 0;
            while i < w_arr.len() {
                let first_cid = w_arr[i].resolve(arena).as_integer().unwrap_or(0) as u32;
                let next_obj = w_arr[i + 1].resolve(arena);
                if let Object::Array(iah) = next_obj {
                    if let Some(i_arr) = arena.get_array(iah) {
                        for (idx, w_obj) in i_arr.iter().enumerate() {
                            let w_val: f32 = w_obj.resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                            metrics.widths.insert(first_cid + idx as u32, w_val);
                        }
                    }
                    i += 2;
                } else {
                    let last_cid = next_obj.as_integer().unwrap_or(0) as u32;
                    let w_val: f32 = w_arr[i + 2].resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                    for cid in first_cid..=last_cid { metrics.widths.insert(cid, w_val); }
                    i += 3;
                }
            }
        }
        metrics.v_widths = Self::parse_v2_metrics(df_dict, arena, metrics.default_width);
        metrics
    }

    fn parse_v2_metrics(df_dict: &BTreeMap<Handle<PdfName>, Object>, arena: &PdfArena, default_w: f32) -> BTreeMap<u32, (f32, f32, f32)> {
        let mut v_widths = BTreeMap::new();
        let Some(Object::Array(wah)) = df_dict.get(&arena.name("W2")).map(|o: &Object| o.resolve(arena)) else { return v_widths };
        let Some(w2_arr) = arena.get_array(wah) else { return v_widths };
        
        let mut i: usize = 0;
        while i < w2_arr.len() {
            let first_cid = w2_arr[i].resolve(arena).as_integer().unwrap_or(0) as u32;
            let next_obj = w2_arr[i + 1].resolve(arena);
            if let Object::Array(iah) = next_obj {
                if let Some(i_arr) = arena.get_array(iah) {
                    for (idx, chunk) in i_arr.chunks_exact(3).enumerate() {
                        let w1_y = chunk[0].resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                        let v_x = chunk[1].resolve(arena).as_f64().unwrap_or(default_w as f64 / 2.0) as f32;
                        let v_y = chunk[2].resolve(arena).as_f64().unwrap_or(880.0) as f32;
                        v_widths.insert(first_cid + idx as u32, (w1_y, v_x, v_y));
                    }
                }
                i += 2;
            } else {
                let last_cid = next_obj.as_integer().unwrap_or(0) as u32;
                let w1_y = w2_arr[i + 2].resolve(arena).as_f64().unwrap_or(1000.0) as f32;
                let v_x = w2_arr[i + 3].resolve(arena).as_f64().unwrap_or(default_w as f64 / 2.0) as f32;
                let v_y = w2_arr[i + 4].resolve(arena).as_f64().unwrap_or(880.0) as f32;
                for cid in first_cid..=last_cid { v_widths.insert(cid, (w1_y, v_x, v_y)); }
                i += 5;
            }
        }
        v_widths
    }

    fn parse_standard_widths(dict: &BTreeMap<Handle<PdfName>, Object>, arena: &PdfArena) -> FontMetrics {
        let mut metrics = FontMetrics {
            first: dict.get(&arena.name("FirstChar")).and_then(|o| o.resolve(arena).as_integer()).unwrap_or(0) as i32,
            last: dict.get(&arena.name("LastChar")).and_then(|o| o.resolve(arena).as_integer()).unwrap_or(0) as i32,
            ..Default::default()
        };
        
        if let Some(Object::Array(ah)) = dict.get(&arena.name("Widths")).map(|o| o.resolve(arena))
            && let Some(arr) = arena.get_array(ah) {
            for (idx, w) in arr.iter().enumerate() {
                metrics.widths.insert((metrics.first + idx as i32) as u32, w.resolve(arena).as_f64().unwrap_or(0.0) as f32);
            }
        }
        metrics
    }

    fn detect_wmode(dict: &BTreeMap<Handle<PdfName>, Object>, arena: &PdfArena) -> i32 {
        let enc_obj = dict.get(&arena.name("Encoding"));
        if let Some(enc) = enc_obj {
            let resolved = enc.resolve(arena);
            match resolved {
                Object::Name(h) => {
                    if let Some(n) = arena.get_name(h)
                        && (n.as_str().ends_with("-V") || n.as_str() == "V") { return 1; }
                }
                Object::Stream(dh, _) => {
                    if let Some(d) = arena.get_dict(dh)
                        && let Some(n) = d.get(&arena.name("CMapName")).and_then(|o| o.resolve(arena).as_name())
                        && let Some(name) = arena.get_name(n)
                        && name.as_str().ends_with("-V") {
                        return 1;
                    }
                }
                _ => {}
            }
        }
        0
    }


    pub fn has_any_mapping(&self) -> bool {
        (self.to_unicode.as_ref().map(|m| !m.mappings.is_empty()).unwrap_or(false))
            || (self.encoding.as_ref().map(|m| !m.mappings.is_empty()).unwrap_or(false))
    }

    /// Loads a Font resource directly from lopdf objects (used during ingest refinement).
    pub fn from_lopdf(
        _id: (u32, u16),
        dict: &lopdf::Dictionary,
        doc: &lopdf::Document,
    ) -> PdfResult<Self> {
        let subtype = dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| PdfName::new(&String::from_utf8_lossy(n)))
            .ok_or_else(|| PdfError::Other("Missing font subtype".into()))?;

        let base_font_raw = dict.get(b"BaseFont")
            .ok()
            .and_then(|o| o.as_name().ok())
            .unwrap_or(b"Unknown");
        
        let base_font_str = crate::refine::text::recover_string(base_font_raw);
        let base_font = PdfName::new(&base_font_str);

        println!("  Loading font: {}", base_font_str);

        let mut to_unicode = None;
        if let Ok(to_uni_obj) = dict.get(b"ToUnicode")
            && let Ok(rid) = to_uni_obj.as_reference()
            && let Ok(to_uni_stream_obj) = doc.get_object(rid)
            && let Ok(to_uni_stream) = to_uni_stream_obj.as_stream()
            && let Ok(data) = to_uni_stream.decompressed_content()
            && let Ok(m) = cmap::CMap::parse(&data)
        {
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
                    if let Ok(enc_stream_obj) = doc.get_object(*rid)
                        && let Ok(enc_stream) = enc_stream_obj.as_stream()
                        && let Ok(data) = enc_stream.decompressed_content()
                        && let Ok(m) = cmap::CMap::parse(&data)
                    {
                        encoding = Some(m);
                    }
                }
                _ => {}
            }
        }

        let mut wmode = 0;
        if let Some(enc_name) = encoding.as_ref().map(|e| e.name())
            && (enc_name.ends_with("-V") || enc_name == "V")
        {
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
            adj1_mapping: None,
            reverse_adj1_mapping: None,
            discovered_mappings: std::sync::Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::new())),
            unicode_to_gid: std::collections::BTreeMap::new(),
        };

        resource.init_adj1_mapping();
        resource.build_unified_map();
        println!("    Done building unified map for {}", resource.base_font.as_str());
        Ok(resource)
    }

    fn init_adj1_mapping(&mut self) {
        let font_name_lower = self.base_font.as_str().to_lowercase();
        let is_japanese = font_name_lower.contains("hira")
            || font_name_lower.contains("koz")
            || font_name_lower.contains("mincho")
            || font_name_lower.contains("明朝")
            || font_name_lower.contains("gothic")
            || font_name_lower.contains("ゴシック")
            || font_name_lower.contains("aj1")
            || self
                .encoding
                .as_ref()
                .map(|e| e.name().contains("UniJIS") || e.name().contains("90ms") || e.name().contains("90pv"))
                .unwrap_or(false);

        if is_japanese
            && let Some(cmap) = cmap::CMap::load_named("Adobe-Japan1-UCS2") {
            let mut reverse = BTreeMap::new();
            for (cid_bytes, uni) in cmap.mappings.iter() {
                if cid_bytes.len() == 2 {
                    let cid = ((cid_bytes[0] as u32) << 8) | (cid_bytes[1] as u32);
                    reverse.insert(uni.clone(), cid);
                }
            }
            self.adj1_mapping = Some(cmap);
            self.reverse_adj1_mapping = Some(reverse);
        }
        
        self.build_unicode_to_gid();
    }

    pub fn build_unicode_to_gid(&mut self) {
        if let Some(ref data) = self.data
            && let Ok(face) = ttf_parser::Face::parse(data, 0) {
            for subtable in face.tables().cmap.iter().flat_map(|t| t.subtables) {
                if subtable.is_unicode() {
                    subtable.codepoints(|cp| {
                        if let Some(gid) = subtable.glyph_index(cp)
                            && let Some(c) = std::char::from_u32(cp) {
                            self.unicode_to_gid.insert(c, gid.0 as u32);
                        }
                    });
                }
            }
        }
    }

    /// Builds a deterministic mapping from Unicode strings to GIDs.
    /// In case of collisions (multiple byte codes mapping to the same Unicode),
    /// it ensures unique identification by using Private Use Area (PUA) codes.
    fn build_unified_map(&mut self) {
        println!("    Building unified map for font: {}...", self.base_font.to_string_lossy());
        let mut map = BTreeMap::new();

        // Use the encoding map as the primary source of characters described in the PDF
        if let Some(ref enc) = self.encoding {
            // Since we can't easily iterate an opaque CMap, we look at common characters
            // and the ToUnicode mapping if it exists.
            // For Identity-H/V fonts, we map CIDs directly to Unicode or PUA.

            // Hardening: For Identity-H/V, the CID is usually the Unicode or at least
            // 1:1 with it.
            if enc.name().contains("Identity") {
                if let Some(ref to_uni) = self.to_unicode {
                    for (bytes, uni) in to_uni.mappings.iter() {
                        let cid: u32 = if bytes.len() == 2 {
                            ((u32::from(bytes[0])) << 8) | u32::from(bytes[1])
                        } else if bytes.len() == 1 {
                            u32::from(bytes[0])
                        } else {
                            continue;
                        };
                        map.insert(uni.clone(), cid);
                    }
                }
            } else {
                if let Some(ref enc) = self.encoding {
                    for (bytes, uni) in enc.mappings.iter() {
                        let cid = enc.to_cid(bytes);
                        if cid > 0 {
                            let unicode = if let Some(ref to_uni) = self.to_unicode {
                                to_uni.map(bytes).unwrap_or_else(|| uni.clone())
                            } else {
                                uni.clone()
                            };
                            map.insert(unicode, cid);
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
            && let Some(fd_dict) = arena.get_dict(fdh)
        {
            // Try FontFile3 (CFF, OpenType, etc.)
            if let Some(ff3) = fd_dict.get(&arena.name("FontFile3"))
                && let Ok(data) = doc.decode_stream(&ff3.resolve(arena))
            {
                return Some(data.to_vec());
            }
            // Try FontFile2 (TrueType)
            if let Some(ff2) = fd_dict.get(&arena.name("FontFile2"))
                && let Ok(data) = doc.decode_stream(&ff2.resolve(arena))
            {
                return Some(data.to_vec());
            }
            // Try FontFile (Type 1)
            if let Some(ff) = fd_dict.get(&arena.name("FontFile"))
                && let Ok(data) = doc.decode_stream(&ff.resolve(arena))
            {
                return Some(data.to_vec());
            }
        }
        None
    }

    pub fn glyph_width(&self, code: &[u8]) -> f32 {
        if code.is_empty() {
            return 0.0;
        }
        let cid = self.to_cid(code);

        if self.wmode == 1 {
            if let Some((w1_y, _, _)) = self.vertical_widths.get(&cid) {
                return *w1_y;
            }
            return 1000.0; // Default vertical advance
        }

        *self.widths.get(&cid).unwrap_or(&self.default_width)
    }

    pub fn generate_standard_tounicode(&self) -> Option<Vec<u8>> {
        let mut cmap = String::new();
        cmap.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap.push_str("12 dict begin\n");
        cmap.push_str("begincmap\n");
        cmap.push_str("/CIDSystemInfo <<\n  /Registry (Adobe)\n  /Ordering (UCS)\n  /Supplement 0\n>> def\n");
        cmap.push_str(&format!("/CMapName /Adobe-Identity-ToUnicode-{} def\n", self.base_font.as_str()));
        cmap.push_str("/CMapType 2 def\n");
        cmap.push_str("1 begincodespacerange\n");
        cmap.push_str("<0000> <FFFF>\n");
        cmap.push_str("endcodespacerange\n");

        // Map GIDs to Unicode
        let mut gid_to_uni = Vec::new();

        // Priority 1: Direct Unicode to GID mapping from font file
        for (&c, &gid) in self.unicode_to_gid.iter() {
            gid_to_uni.push((gid, c.to_string()));
        }

        // Priority 2: Unified map (discovered during ingestion, including Adobe-Japan1)
        if gid_to_uni.is_empty() {
            for (uni_str, &gid) in self.unified_map.iter() {
                gid_to_uni.push((gid, uni_str.clone()));
            }
        }

        if !gid_to_uni.is_empty() {
            gid_to_uni.sort_by_key(|&(gid, _)| gid);
            gid_to_uni.dedup_by_key(|item| item.0);

            // PDF CMap limits begincidchar blocks to 100 entries each.
            for chunk in gid_to_uni.chunks(100) {
                cmap.push_str(&format!("{} begincidchar\n", chunk.len()));
                for &(gid, ref uni_str) in chunk {
                    let gid_hex = format!("{:04X}", gid);
                    let mut uni_hex = String::new();
                    for c in uni_str.chars() {
                        let u = c as u32;
                        if u > 0xFFFF {
                            // Surrogate pair for non-BMP characters
                            let high = 0xD800 + ((u - 0x10000) >> 10);
                            let low = 0xDC00 + ((u - 0x10000) & 0x3FF);
                            uni_hex.push_str(&format!("{:04X}{:04X}", high, low));
                        } else {
                            uni_hex.push_str(&format!("{:04X}", u));
                        }
                    }
                    cmap.push_str(&format!("<{}> <{}>\n", gid_hex, uni_hex));
                }
                cmap.push_str("endcidchar\n");
            }
        }

        cmap.push_str("endcmap\n");
        cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap.push_str("end\nend\n");

        Some(cmap.into_bytes())
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
        self.to_unicode_inner(code)
    }

    fn to_unicode_inner(&self, code: &[u8]) -> Option<String> {
        if let Some(ref map) = self.to_unicode
            && let Some(res) = map.map(code) {
            return Some(res);
        }

        if let Some(ref enc) = self.encoding
            && let Some(res) = enc.map(code) {
            if res.starts_with('/') {
                return Some(cmap::glyph_name_to_unicode(res.as_bytes()));
            }
            return Some(res);
        }

        let cid = self.to_cid(code);

        if let Some(ref adj1) = self.adj1_mapping {
            let cid_bytes = vec![(cid >> 8) as u8, (cid & 0xFF) as u8];
            if let Some(s) = adj1.map(&cid_bytes) {
                return Some(s);
            }
        }

        // Final fallback: if CID is in ASCII range, try to interpret it as a character
        if cid < 128 && cid > 31 {
            return Some((cid as u8 as char).to_string());
        }

        // Positional preservation using Plane 15 PUA
        // This is safe because it's exactly 1 char, but let's prefer Unicode if possible.
        std::char::from_u32(0xF0000 + cid).map(|c| c.to_string())
    }

    pub fn wmode(&self) -> i32 {
        self.wmode
    }

    /// Inferred bold status based on font name and descriptors.
    pub fn is_bold(&self) -> bool {
        let name = self.base_font.as_str().to_lowercase();
        name.contains("bold")
            || name.contains("heavy")
            || name.contains("black")
            || name.contains("-w6")
            || name.contains("-w7")
            || name.contains("-w8")
            || name.contains("-w9")
    }

    /// Returns the width of a glyph in 1/1000 font units, by CID.
    pub fn glyph_width_by_cid(&self, cid: u32) -> f32 {
        self.widths.get(&cid).copied().unwrap_or(self.default_width)
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

    pub fn to_gid(&self, cid: u32) -> u32 {
        if let Some(ref map) = self.cid_to_gid_map {
            return map.get(cid as usize).copied().unwrap_or(cid as u16) as u32;
        }
        cid
    }

    pub fn decode_next(&self, data: &[u8]) -> (usize, Option<String>) {
        if data.is_empty() {
            return (0, None);
        }

        let subtype = self.subtype.as_str();
        let is_multibyte =
            subtype == "Type0" || subtype == "CIDFontType0" || subtype == "CIDFontType2";
        let is_identity =
            self.encoding.as_ref().map(|e| e.name.contains("Identity")).unwrap_or(false);
        let min_len = if is_multibyte || is_identity { Some(2) } else { None };

        // 1. Try ToUnicode Map strictly
        if let Some(tu) = &self.to_unicode
            && let Some((len, u)) = tu.decode_next_with_min_len(data, min_len)
        {
            if let Some(u_str) = u {
                if u_str == "\u{FFFD}" {
                    println!("      [DEBUG] ToUnicode returned FFFD for bytes: {:02X?}", &data[..len]);
                }
                return (len, Some(u_str));
            }
            return (len, None);
        }

        // 2. Try Encoding Map strictly
        if let Some(enc) = &self.encoding
            && let Some((len, u)) = enc.decode_next_with_min_len(data, min_len)
        {
            if let Some(u_str) = u {
                if u_str == "\u{FFFD}" {
                    println!("      [DEBUG] Encoding returned FFFD for bytes: {:02X?}", &data[..len]);
                }
                if u_str.starts_with('/') {
                    return (len, Some(cmap::glyph_name_to_unicode(u_str.as_bytes())));
                }
                return (len, Some(u_str));
            }
            return (len, None);
        }

        // 3. Fallback to Heuristic consumption length
        let subtype = self.subtype.as_str();
        let is_multibyte =
            subtype == "Type0" || subtype == "CIDFontType0" || subtype == "CIDFontType2";
        let is_identity =
            self.encoding.as_ref().map(|e| e.name.contains("Identity")).unwrap_or(false);

        // Hardening: Suppress heuristic if font is known to repurpose ASCII codes (Legacy Distiller)
        if !is_multibyte && !is_identity && !self.is_legacy_distiller && !data.is_empty() {
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
        let is_simple =
            subtype_str == "Type1" || subtype_str == "TrueType" || subtype_str == "Type3";
        let has_reliable_map = self.to_unicode.is_some() || self.encoding.is_some();

        if is_simple
            && !self.is_legacy_distiller
            && !has_reliable_map
            && consumed == 1
            && code_bytes[0] >= 0x20
            && code_bytes[0] <= 0x7E
        {
            return (consumed, Some((code_bytes[0] as char).to_string()));
        }

        // 5. Hard fallback for Identity-H/V
        if is_identity && consumed == 2 {
            // Final fallback: Use to_unicode() which includes Adobe-Japan1 resource mapping
            return (consumed, self.to_unicode(code_bytes));
        }

        (consumed, self.to_unicode(code_bytes))
    }

    pub fn generate_tounicode_from_utf8(&self) -> Option<Vec<u8>> {
        let mut cmap = String::new();
        cmap.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap.push_str("12 dict begin\n");
        cmap.push_str("begincmap\n");
        cmap.push_str("/CIDSystemInfo <<\n  /Registry (Adobe)\n  /Ordering (UCS)\n  /Supplement 0\n>> def\n");
        cmap.push_str(&format!("/CMapName /Adobe-UTF8-ToUnicode-{} def\n", self.base_font.as_str()));
        cmap.push_str("/CMapType 2 def\n");
        cmap.push_str("4 begincodespacerange\n");
        cmap.push_str("<00> <7F>\n");
        cmap.push_str("<C280> <DFBF>\n");
        cmap.push_str("<E0A080> <EFBFBF>\n");
        cmap.push_str("<F0908080> <F48FBFBF>\n");
        cmap.push_str("endcodespacerange\n");

        if !self.unicode_to_gid.is_empty() {
            cmap.push_str(&format!("{} begincidchar\n", self.unicode_to_gid.len()));
            for (&c, _) in self.unicode_to_gid.iter() {
                let mut buf = [0u8; 4];
                let utf8_bytes = c.encode_utf8(&mut buf).as_bytes();
                let utf8_hex = utf8_bytes.iter().map(|b| format!("{:02X}", b)).collect::<String>();
                let uni_hex = format!("{:04X}", c as u32);
                cmap.push_str(&format!("<{}> <{}>\n", utf8_hex, uni_hex));
            }
            cmap.push_str("endcidchar\n");
        }

        cmap.push_str("endcmap\n");
        cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap.push_str("end\nend\n");

        Some(cmap.into_bytes())
    }
}

pub fn list_fonts(doc: &Document) -> Vec<FontSummary> {
    let arena = doc.arena();
    let mut fonts = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    let type_key = arena.get_name_by_str("Type");
    let font_val = arena.get_name_by_str("Font");

    if let (Some(tk), Some(fv)) = (type_key, font_val) {
        for handle in arena.all_dict_handles() {
            if let Some(dict) = arena.get_dict(handle)
                && dict.get(&tk).and_then(|o| o.resolve(arena).as_name()) == Some(fv)
            {
                if seen.contains(&handle) {
                    continue;
                }
                seen.insert(handle);
                if let Some(summary) = extract_font_summary(arena, &dict, fv) {
                    fonts.push(summary);
                }
            }
        }
    }
    fonts
}

fn extract_font_summary(
    arena: &PdfArena,
    dict: &std::collections::BTreeMap<crate::handle::Handle<crate::object::PdfName>, Object>,
    fv: crate::handle::Handle<crate::object::PdfName>,
) -> Option<FontSummary> {
    let base_font_key = arena.get_name_by_str("BaseFont");
    let subtype_key = arena.get_name_by_str("Subtype");
    let encoding_key = arena.get_name_by_str("Encoding");
    let desc_key = arena.get_name_by_str("FontDescriptor");
    let f1_key = arena.get_name_by_str("FontFile");
    let f2_key = arena.get_name_by_str("FontFile2");
    let f3_key = arena.get_name_by_str("FontFile3");

    // 1. Try BaseFont as Name or String
    let mut name = dict
        .get(&base_font_key.unwrap_or(fv))
        .and_then(|o| {
            let res = o.resolve(arena);
            match res {
                Object::Name(h) => {
                    let raw = arena.get_name(h)?.as_bytes().to_vec();
                    Some(crate::refine::text::recover_string(&raw))
                }
                Object::String(s) => Some(crate::refine::text::recover_string(&s)),
                _ => None,
            }
        })
        .unwrap_or_else(|| "Untitled".to_string());

    let font_type = dict
        .get(&subtype_key.unwrap_or(fv))
        .and_then(|o| o.resolve(arena).as_name())
        .and_then(|n| arena.get_name_str(n))
        .unwrap_or_else(|| "Type1".to_string());

    if (name == "Untitled" || name.is_empty() || name.contains('\u{FFFD}'))
        && let Some(fd_obj) = dict.get(&arena.name("FontDescriptor"))
        && let Some(fd_dict) = fd_obj.resolve(arena).as_dict_handle().and_then(|dh| arena.get_dict(dh))
        && let Some(fn_val) = fd_dict.get(&arena.name("FontName")).and_then(|o| o.resolve(arena).as_name())
    {
        let raw = arena.get_name(fn_val).map(|n| n.as_bytes().to_vec()).unwrap_or_default();
        if !raw.is_empty() {
            name = crate::refine::text::recover_string(&raw);
        }
    }

    // 3. For Type0 fonts, peek into descendants
    if (name == "Untitled" || name.is_empty())
        && font_type == "Type0"
        && let Some(dk) = arena.get_name_by_str("DescendantFonts")
        && let Some(kids_obj) = dict.get(&dk)
        && let Some(kids) = kids_obj.resolve(arena).as_array().and_then(|ah| arena.get_array(ah))
        && let Some(kid) = kids.first()
        && let Some(kdh) = kid.resolve(arena).as_dict_handle()
        && let Some(kdict) = arena.get_dict(kdh)
        && let Some(bf) = kdict.get(&base_font_key.unwrap_or(fv)).and_then(|o| {
            let res = o.resolve(arena);
            match res {
                Object::Name(h) => {
                    let raw = arena.get_name(h)?.as_bytes().to_vec();
                    Some(crate::refine::text::recover_string(&raw))
                }
                Object::String(s) => Some(crate::refine::text::recover_string(&s)),
                _ => None,
            }
        })
    {
        name = bf;
    }

    // 4. Final fallback to Name alias
    if (name == "Untitled" || name.is_empty())
        && let Some(n) = dict
            .get(&arena.name("Name"))
            .and_then(|o| o.resolve(arena).as_name())
            .and_then(|h| arena.get_name_str(h))
    {
        name = n;
    }

    let encoding_obj = dict.get(&encoding_key.unwrap_or(fv)).map(|o| o.resolve(arena));
    let encoding = match encoding_obj {
        Some(Object::Name(h)) => arena.get_name_str(h).unwrap_or_else(|| "CustomName".to_string()),
        Some(Object::Dictionary(_)) => "CustomDict".to_string(),
        Some(Object::Stream(_, _)) => "CustomStream".to_string(),
        _ => "Standard".to_string(),
    };

    let mut is_embedded = false;
    if let Some(desc_handle) = dict
        .get(&desc_key.unwrap_or(fv))
        .and_then(|o| o.resolve(arena).as_dict_handle())
        && let Some(desc_dict) = arena.get_dict(desc_handle)
        && [f1_key, f2_key, f3_key].iter().flatten().any(|k| desc_dict.contains_key(k))
    {
        is_embedded = true;
    }

    let is_subset = name.len() > 7 && name.as_bytes().get(6).copied() == Some(b'+');

    Some(FontSummary { name, font_type, is_embedded, is_subset, encoding })
}
