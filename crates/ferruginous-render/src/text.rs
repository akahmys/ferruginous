use kurbo::{BezPath, Point};
use skrifa::outline::{OutlinePen, DrawSettings};
use skrifa::instance::{Size, LocationRef};
use skrifa::{FontRef, MetadataProvider, GlyphId};

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
        self.path.curve_to(Point::new(x1 as f64, y1 as f64), Point::new(x2 as f64, y2 as f64), Point::new(x as f64, y as f64));
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

pub struct SkrifaBridge {}

impl Default for SkrifaBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl SkrifaBridge {
    pub fn new() -> Self {
        Self {}
    }
    pub fn extract_path(&self, font_data: &[u8], glyph_id: u32) -> Option<BezPath> {
        let font_ref = FontRef::new(font_data).ok()?;
        let outlines = font_ref.outline_glyphs();
        // Try direct construction
        let gid = GlyphId::from(glyph_id as u16);
        let glyph = outlines.get(gid)?;
        
        let mut pen = KurboPen::new();
        // Try fallback size if unscaled is not found
        let settings = DrawSettings::unhinted(Size::new(1000.0), LocationRef::default());
        glyph.draw(settings, &mut pen).ok()?;
        
        Some(pen.finish())
    }
}
