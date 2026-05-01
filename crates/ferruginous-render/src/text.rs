use kurbo::{BezPath, Point};
use skrifa::MetadataProvider;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::{FileRef, TableProvider};
use std::collections::BTreeMap;

pub struct KurboPen {
    path: BezPath,
}

impl Default for KurboPen {
    fn default() -> Self {
        Self::new()
    }
}

impl KurboPen {
    pub fn new() -> Self {
        Self { path: BezPath::new() }
    }
    pub fn finish(self) -> BezPath {
        self.path
    }
}

impl OutlinePen for KurboPen {
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
        self.path.curve_to(
            Point::new(x1 as f64, y1 as f64),
            Point::new(x2 as f64, y2 as f64),
            Point::new(x as f64, y as f64),
        );
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

pub struct TextLayoutOptions {
    pub font_size: f32,
    pub char_spacing: f32,
    pub word_spacing: f32,
    pub horizontal_scaling: f32,
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self { font_size: 1.0, char_spacing: 0.0, word_spacing: 0.0, horizontal_scaling: 100.0 }
    }
}

pub struct SkrifaBridge {
    glyph_cache: BTreeMap<(u64, u32, u32), BezPath>,
}

impl SkrifaBridge {
    pub fn new() -> Self {
        Self { glyph_cache: BTreeMap::new() }
    }

    pub fn get_units_per_em(&self, data: &[u8]) -> Option<u16> {
        if let Ok(file) = FileRef::new(data) {
            let font = match file {
                FileRef::Font(f) => f,
                FileRef::Collection(c) => c.get(0).ok()?,
            };
            return Some(font.head().ok()?.units_per_em());
        }
        None
    }
}

pub struct GlyphExtractionContext<'a> {
    pub font_id: u64,
    pub data: &'a [u8],
    pub gid: u32,
    pub char_code: u32,
    pub cid_to_gid_map: Option<&'a [u16]>,
    pub is_vertical: bool,
    pub unicode_fallback: Option<char>,
    pub is_japanese: bool,
    pub is_cid: bool,
    pub collection_index: u32,
}

impl SkrifaBridge {
    pub fn extract_path(&mut self, ctx: &GlyphExtractionContext) -> Option<BezPath> {
        if let Some(path) = self.glyph_cache.get(&(ctx.font_id, ctx.gid, ctx.char_code)) {
            return Some(path.clone());
        }

        let final_gid = ctx.gid;

        // Normalize-at-Load ensures that ctx.data contains either the embedded font data
        // or the resolved system fallback data. The renderer no longer performs heuristics.
        let path = self.try_extract_from_data(
            ctx.data,
            final_gid,
            ctx.char_code,
            ctx.is_cid,
            ctx.collection_index,
            ctx.unicode_fallback,
        );

        if path.is_none() && !self.is_blank_char(ctx.unicode_fallback) {
            // Optional: log as debug in a real app
        }

        if let Some(ref p) = path {
            if p.segments().count() > 0 {
                self.glyph_cache.insert((ctx.font_id, ctx.gid, ctx.char_code), p.clone());
            }
        }
        path
    }

    fn is_blank_char(&self, u: Option<char>) -> bool {
        match u {
            Some('\u{0020}')
            | Some('\u{00A0}')
            | Some('\u{2000}'..='\u{200F}')
            | Some('\u{3000}')
            | Some('\u{202F}') => true,
            _ => false,
        }
    }

    fn try_extract_from_data(
        &self,
        data: &[u8],
        final_gid: u32,
        char_code: u32,
        is_cid: bool,
        collection_index: u32,
        unicode: Option<char>,
    ) -> Option<BezPath> {
        if data.is_empty() {
            return None;
        }

        let font = match skrifa::raw::FontRef::from_index(data, collection_index) {
            Ok(f) => f,
            Err(_e) => {
                return None;
            }
        };

        let gid = if is_cid {
            skrifa::GlyphId::new(final_gid)
        } else if let Some(u) = unicode
            && !(u >= '\u{E000}' && u <= '\u{F8FF}')
            && !(u >= '\u{F0000}' && u <= '\u{FFFFD}')
        {
            font.charmap().map(u).unwrap_or_else(|| {
                font.charmap().map(char_code).unwrap_or_else(|| skrifa::GlyphId::new(final_gid))
            })
        } else {
            font.charmap().map(char_code).unwrap_or_else(|| skrifa::GlyphId::new(final_gid))
        };

        let upem = font.head().map(|h| h.units_per_em()).unwrap_or(1000);
        let mut pen = KurboPen::new();
        let Some(glyph) = font.outline_glyphs().get(gid) else {
            return None;
        };
        if let Err(_e) = glyph
            .draw(DrawSettings::unhinted(Size::new(upem as f32), LocationRef::default()), &mut pen)
        {
            return None;
        }
        let path = pen.finish();
        if path.segments().count() == 0 && !self.is_blank_char(unicode) {
            return None;
        }
        Some(path)
    }
}
