use kurbo::{BezPath, Point};
use skrifa::outline::{OutlinePen, DrawSettings};
use skrifa::instance::{Size, LocationRef};
use skrifa::{MetadataProvider, GlyphId};
use skrifa::raw::FileRef;

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

pub struct TextLayoutOptions {
    pub font_size: f32,
    pub char_spacing: f32,
    pub word_spacing: f32,
    pub horizontal_scaling: f32, // Percentage (100.0)
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self {
            font_size: 1.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
        }
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

    pub fn extract_path(&self, data: &[u8], gid: u32) -> Option<BezPath> {
        let file = FileRef::new(data).ok()?;
        let font = match file {
            FileRef::Font(f) => Some(f),
            FileRef::Collection(c) => {
                // Try to find a font in the collection that has this glyph
                for i in 0..c.len() {
                    if let Ok(f) = c.get(i) {
                        let mut pen = KurboPen::new();
                        let settings = DrawSettings::unhinted(Size::new(1000.0), LocationRef::default());
                        let outlines = f.outline_glyphs();
                        if let Some(g) = outlines.get(GlyphId::new(gid))
                            && g.draw(settings, &mut pen).is_ok() {
                                let path = pen.finish();
                                if !path.is_empty() {
                                    return Some(path);
                                }
                            }
                    }
                }
                c.get(0).ok()
            }
        }?;
        
        let outlines = font.outline_glyphs();
        let glyph = outlines.get(GlyphId::new(gid))?;
        
        let mut pen = KurboPen::new();
        // PDF fonts are usually 1000 units per em
        let settings = DrawSettings::unhinted(Size::new(1000.0), LocationRef::default());
        glyph.draw(settings, &mut pen).ok()?;
        
        Some(pen.finish())
    }

    /// Renders a sequence of glyphs into a single path, applying PDF-specific layout rules.
    pub fn render_glyphs(
        &self, 
        font_data: &[u8], 
        glyphs: &[(u32, f32)], // (GlyphId, Width override if any)
        options: &TextLayoutOptions
    ) -> BezPath {
        let mut combined_path = BezPath::new();
        let mut x_offset = 0.0;
        
        let scale = options.font_size / 1000.0;
        let h_scale = options.horizontal_scaling / 100.0;

        for (gid, width) in glyphs {
            if let Some(path) = self.extract_path(font_data, *gid) {
                // Scale and position the glyph
                let transform = kurbo::Affine::translate((x_offset, 0.0))
                    * kurbo::Affine::scale_non_uniform(scale as f64 * h_scale as f64, scale as f64);
                
                // Apply transform to the glyph path
                let mut path = path;
                path.apply_affine(transform);
                combined_path.extend(path);
            }
            
            // Update x_offset (ISO 32000 Section 9.4.4)
            // tx = ((w0 - Tj/1000) * Th + Tc + Tw) * Tf
            x_offset += *width as f64 * scale as f64 * h_scale as f64;
            x_offset += options.char_spacing as f64 * h_scale as f64;

            if *gid == 32 {
                x_offset += options.word_spacing as f64 * h_scale as f64;
            }
        }
        
        combined_path
    }
}
