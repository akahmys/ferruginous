use kurbo::{BezPath, Point, Affine};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::{FileRef, TableProvider};
use skrifa::MetadataProvider;
use std::collections::BTreeMap;

pub struct KurboPen {
    path: BezPath,
}

impl Default for KurboPen {
    fn default() -> Self { Self::new() }
}

impl KurboPen {
    pub fn new() -> Self { Self { path: BezPath::new() } }
    pub fn finish(self) -> BezPath { self.path }
}

impl OutlinePen for KurboPen {
    fn move_to(&mut self, x: f32, y: f32) { self.path.move_to(Point::new(x as f64, y as f64)); }
    fn line_to(&mut self, x: f32, y: f32) { self.path.line_to(Point::new(x as f64, y as f64)); }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) { self.path.quad_to(Point::new(x1 as f64, y1 as f64), Point::new(x as f64, y as f64)); }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.curve_to(Point::new(x1 as f64, y1 as f64), Point::new(x2 as f64, y2 as f64), Point::new(x as f64, y as f64));
    }
    fn close(&mut self) { self.path.close_path(); }
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
    pub primary_system_font: Option<Vec<u8>>,
    glyph_cache: BTreeMap<(u32, u32), BezPath>,
}

impl SkrifaBridge {
    pub fn new(primary_system_font: Option<Vec<u8>>) -> Self {
        Self { primary_system_font, glyph_cache: BTreeMap::new() }
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

    pub fn extract_path(
        &mut self,
        data: &[u8],
        gid: u32,
        char_code: u32,
        _cid_to_gid_map: Option<&[u16]>,
        _is_vertical: bool,
        _unicode_fallback: Option<char>,
    ) -> Option<BezPath> {
        if let Some(path) = self.glyph_cache.get(&(gid, char_code)) {
            return Some(path.clone());
        }

        if let Ok(file) = FileRef::new(data) {
            let font = match file {
                FileRef::Font(f) => f,
                FileRef::Collection(c) => c.get(0).ok()?,
            };
            
            let mut pen = KurboPen::new();
            let glyph = font.outline_glyphs().get(skrifa::GlyphId::new(gid))?;
            if glyph.draw(DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()), &mut pen).is_ok() {
                let path = pen.finish();
                self.glyph_cache.insert((gid, char_code), path.clone());
                return Some(path);
            }
        }
        None
    }
}

impl Default for SkrifaBridge {
    fn default() -> Self { Self::new(None) }
}
