//! Ferruginous Render: Graphics Bridging for PDF.
//!
//! (ISO 32000-2:2020 Clause 8)

pub mod ctm;
pub mod path;
pub mod text;
pub mod headless;

#[cfg(test)]
mod path_tests;
#[cfg(test)]
mod text_tests;

use vello::Scene;
use kurbo::{Affine, BezPath, Stroke, Cap, Join};
use ferruginous_core::graphics::{Color, WindingRule, StrokeStyle, LineCap, LineJoin, PixelFormat};
use skrifa::MetadataProvider;

pub trait RenderBackend: Send {
    fn push_state(&mut self);
    fn pop_state(&mut self);
    fn transform(&mut self, affine: Affine);
    
    fn fill_path(&mut self, path: &BezPath, color: &Color, rule: WindingRule);
    fn stroke_path(&mut self, path: &BezPath, color: &Color, style: &StrokeStyle);
    
    fn push_clip(&mut self, path: &BezPath, rule: WindingRule);
    fn pop_clip(&mut self);
    fn draw_image(&mut self, data: &[u8], width: u32, height: u32, format: PixelFormat);

    // Transparency Support
    fn set_fill_alpha(&mut self, alpha: f64);
    fn set_stroke_alpha(&mut self, alpha: f64);
    fn set_blend_mode(&mut self, mode: ferruginous_core::graphics::BlendMode);

    // Color Support
    fn set_fill_color(&mut self, color: Color);
    fn set_stroke_color(&mut self, color: Color);

    fn define_font(&mut self, name: &str, data: Vec<u8>);
    fn show_text(&mut self, text: &str, font_name: &str, size: f32, transform: Affine);
}

/// Internal graphics state for VelloBackend.
#[derive(Clone)]
struct VelloState {
    transform: Affine,
    fill_color: Color,
    stroke_color: Color,
    fill_alpha: f64,
    stroke_alpha: f64,
    blend_mode: ferruginous_core::graphics::BlendMode,
    clip_count: usize,
}

/// Vello-based implementation of [RenderBackend].
pub struct VelloBackend {
    scene: Scene,
    state: VelloState,
    state_stack: Vec<VelloState>,
    font_cache: std::collections::BTreeMap<String, (Vec<u8>, Option<u32>)>, // Name -> (Data, CollectionIndex)
}

impl Default for VelloBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl VelloBackend {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            state: VelloState {
                transform: Affine::IDENTITY,
                fill_color: Color::Gray(0.0),
                stroke_color: Color::Gray(0.0),
                fill_alpha: 1.0,
                stroke_alpha: 1.0,
                blend_mode: ferruginous_core::graphics::BlendMode::Normal,
                clip_count: 0,
            },
            state_stack: Vec::new(),
            font_cache: std::collections::BTreeMap::new(),
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }
}

impl RenderBackend for VelloBackend {
    fn push_state(&mut self) {
        self.state_stack.push(self.state.clone());
        self.state.clip_count = 0; // Reset clip count for the new level
    }
    
    fn pop_state(&mut self) {
        // Pop all clips pushed at this level
        for _ in 0..self.state.clip_count {
            self.scene.pop_layer();
        }

        if let Some(previous_state) = self.state_stack.pop() {
            self.state = previous_state;
        }
    }
    
    fn transform(&mut self, affine: Affine) {
        self.state.transform *= affine;
    }
    
    fn fill_path(&mut self, path: &BezPath, color: &Color, rule: WindingRule) {
        // Resource Limit: Prevention of OOM from overly complex paths
        if path.segments().count() > 100_000 {
            eprintln!("WARNING: Path complexity limit exceeded, skipping fill.");
            return;
        }

        let brush = to_vello_brush(color, self.state.fill_alpha as f32);
        let fill = match rule {
            WindingRule::NonZero => vello::peniko::Fill::NonZero,
            WindingRule::EvenOdd => vello::peniko::Fill::EvenOdd,
        };
        self.scene.fill(fill, self.state.transform, &brush, None, path);
    }
    
    fn stroke_path(&mut self, path: &BezPath, color: &Color, style: &StrokeStyle) {
        // Resource Limit: Prevention of OOM from overly complex paths
        if path.segments().count() > 100_000 {
            eprintln!("WARNING: Path complexity limit exceeded, skipping stroke.");
            return;
        }

        let brush = to_vello_brush(color, self.state.stroke_alpha as f32);
        let cap = match style.cap {
            LineCap::Butt => Cap::Butt,
            LineCap::Round => Cap::Round,
            LineCap::Square => Cap::Square,
        };
        let join = match style.join {
            LineJoin::Miter => Join::Miter,
            LineJoin::Round => Join::Round,
            LineJoin::Bevel => Join::Bevel,
        };
        
        let (dash_offset, is_dashed) = if let Some((_, phase)) = &style.dash_pattern {
            (*phase, true)
        } else {
            (0.0, false)
        };

        // Use a more robust way to create Stroke to avoid field mismatch
        let mut stroke = kurbo::Stroke::new(style.width);
        stroke.start_cap = cap;
        stroke.end_cap = cap;
        stroke.join = join;
        stroke.miter_limit = style.miter_limit;
        stroke.dash_offset = dash_offset;
        
        if is_dashed {
             if let Some((pattern, phase)) = &style.dash_pattern {
                 let dashed: BezPath = kurbo::dash(path.iter(), *phase, pattern).collect();
                 self.scene.stroke(&stroke, self.state.transform, &brush, None, &dashed);
             }
        } else {
             self.scene.stroke(&stroke, self.state.transform, &brush, None, path);
        }
    }
    
    fn push_clip(&mut self, path: &BezPath, rule: WindingRule) {
        let fill = match rule {
            WindingRule::NonZero => vello::peniko::Fill::NonZero,
            WindingRule::EvenOdd => vello::peniko::Fill::EvenOdd,
        };
        // Use push_layer for clipping with proper fill
        self.scene.push_layer(fill, vello::peniko::Mix::Normal, 1.0, self.state.transform, path);
        self.state.clip_count += 1;
    }
    
    fn pop_clip(&mut self) {
        if self.state.clip_count > 0 {
            self.scene.pop_layer();
            self.state.clip_count -= 1;
        }
    }
    
    fn draw_image(&mut self, data: &[u8], width: u32, height: u32, format: PixelFormat) {
        use vello::peniko::{Blob, ImageFormat, ImageData, ImageAlphaType};

        // Convert input data to RGBA8 for Vello compatibility and to prevent buffer overruns
        let rgba_data = match format {
            PixelFormat::Gray8 => {
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                let expected_len = width as usize * height as usize;
                for i in 0..expected_len {
                    let g = data.get(i).copied().unwrap_or(0);
                    rgba.extend_from_slice(&[g, g, g, 255]);
                }
                rgba
            }
            PixelFormat::Rgb8 => {
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                let expected_len = width as usize * height as usize;
                for i in 0..expected_len {
                    let r = data.get(i * 3).copied().unwrap_or(0);
                    let g = data.get(i * 3 + 1).copied().unwrap_or(0);
                    let b = data.get(i * 3 + 2).copied().unwrap_or(0);
                    rgba.extend_from_slice(&[r, g, b, 255]);
                }
                rgba
            }
            PixelFormat::Cmyk8 => {
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                let expected_len = width as usize * height as usize;
                for i in 0..expected_len {
                    let c = data.get(i * 4).copied().map(|v| v as f32 / 255.0).unwrap_or(0.0);
                    let m = data.get(i * 4 + 1).copied().map(|v| v as f32 / 255.0).unwrap_or(0.0);
                    let y = data.get(i * 4 + 2).copied().map(|v| v as f32 / 255.0).unwrap_or(0.0);
                    let k = data.get(i * 4 + 3).copied().map(|v| v as f32 / 255.0).unwrap_or(0.0);
                    let r = (1.0 - c) * (1.0 - k);
                    let g = (1.0 - m) * (1.0 - k);
                    let b = (1.0 - y) * (1.0 - k);
                    rgba.extend_from_slice(&[(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]);
                }
                rgba
            }
        };

        let blob_data: std::sync::Arc<dyn AsRef<[u8]> + Send + Sync> = std::sync::Arc::new(rgba_data);
        let image = ImageData {
            data: Blob::new(blob_data),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width,
            height,
        };
        
        self.scene.draw_image(&image, self.state.transform);
    }

    fn set_fill_alpha(&mut self, alpha: f64) {
        self.state.fill_alpha = alpha;
    }

    fn set_stroke_alpha(&mut self, alpha: f64) {
        self.state.stroke_alpha = alpha;
    }

    fn set_blend_mode(&mut self, mode: ferruginous_core::graphics::BlendMode) {
        self.state.blend_mode = mode;
    }

    fn set_fill_color(&mut self, color: Color) {
        self.state.fill_color = color;
    }
    
    fn set_stroke_color(&mut self, color: Color) {
        self.state.stroke_color = color;
    }

    fn define_font(&mut self, name: &str, data: Vec<u8>) {
        // Pre-scan for Japanese support if it's a TTC
        let index = if let Ok(file_ref) = skrifa::raw::FileRef::new(&data) {
            match file_ref {
                skrifa::raw::FileRef::Font(_) => None,
                skrifa::raw::FileRef::Collection(c) => {
                    // Find a font with Japanese support or just default to 0
                    let mut best_index = 0;
                    for i in 0..c.len() {
                        if let Ok(_font) = c.get(i) {
                            // Quick check for Hiragino/Japanese-ish
                            // In a real impl, we'd check Name table
                            best_index = i;
                            break;
                        }
                    }
                    Some(best_index)
                }
            }
        } else {
            None
        };
        self.font_cache.insert(name.to_string(), (data, index));
    }

    fn show_text(&mut self, text: &str, font_name: &str, size: f32, transform: Affine) {
        // 1. Try Cache
        let (font_data, ttc_index) = if let Some(cached) = self.font_cache.get(font_name) {
            (cached.0.clone(), cached.1)
        } else {
            // Fallback for diagnostic/legacy
            let fallback_path = "/System/Library/Fonts/Hiragino Sans GB.ttc";
            match std::fs::read(fallback_path) {
                Ok(data) => (data, Some(0)),
                Err(_) => return,
            }
        };

        if let Ok(file_ref) = skrifa::raw::FileRef::new(&font_data) {
            let font = match file_ref {
                skrifa::raw::FileRef::Font(f) => Some(f),
                skrifa::raw::FileRef::Collection(c) => c.get(ttc_index.unwrap_or(0)).ok(),
            };

            if let Some(font) = font {
                let mut x_offset = 0.0;
                let scale = size as f64 / 1000.0;
                let bridge = crate::text::SkrifaBridge::new();
                let brush = to_vello_brush(&self.state.fill_color, self.state.fill_alpha as f32);
                
                for c in text.chars() {
                    let gid = font.charmap().map(c).unwrap_or(skrifa::GlyphId::new(0));
                    let glyph_transform = self.state.transform 
                        * transform 
                        * kurbo::Affine::translate((x_offset, 0.0))
                        * kurbo::Affine::scale_non_uniform(scale, scale);
                    
                    if let Some(mut path) = bridge.extract_path(&font_data, gid.to_u32()) {
                        path.apply_affine(glyph_transform);
                        self.scene.fill(vello::peniko::Fill::NonZero, Affine::IDENTITY, &brush, None, &path);
                        
                        let metrics = font.glyph_metrics(skrifa::instance::Size::new(1000.0), skrifa::instance::LocationRef::default());
                        let advance = metrics.advance_width(gid).unwrap_or(0.0) as f64 * scale;
                        
                        x_offset += advance;
                    }
                }
            } else {
                eprintln!("DEBUG: show_text fallback: No font found");
            }
        } else {
            eprintln!("DEBUG: show_text fallback: Failed to parse font");
        }
    }
}

#[allow(dead_code)]
fn to_kurbo_stroke(style: &StrokeStyle) -> kurbo::Stroke {
    let mut stroke = Stroke::new(style.width);
    let cap = match style.cap {
        LineCap::Butt => Cap::Butt,
        LineCap::Round => Cap::Round,
        LineCap::Square => Cap::Square,
    };
    stroke.start_cap = cap;
    stroke.end_cap = cap;
    
    stroke.join = match style.join {
        LineJoin::Miter => Join::Miter,
        LineJoin::Round => Join::Round,
        LineJoin::Bevel => Join::Bevel,
    };
    stroke.miter_limit = style.miter_limit;
    stroke
}

fn to_vello_brush(color: &Color, alpha: f32) -> vello::peniko::Brush {
    match color {
        Color::Gray(g) => vello::peniko::Brush::Solid(vello::peniko::Color::new([*g as f32, *g as f32, *g as f32, alpha])),
        Color::Rgb(r, g, b) => vello::peniko::Brush::Solid(vello::peniko::Color::new([*r as f32, *g as f32, *b as f32, alpha])),
        Color::Cmyk(c, m, y, k) => {
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);
            vello::peniko::Brush::Solid(vello::peniko::Color::new([r as f32, g as f32, b as f32, alpha]))
        }
    }
}

#[allow(dead_code)]
fn to_vello_mix(mode: ferruginous_core::graphics::BlendMode) -> vello::peniko::Mix {
    use ferruginous_core::graphics::BlendMode;
    use vello::peniko::Mix;
    match mode {
        BlendMode::Normal => Mix::Normal,
        BlendMode::Multiply => Mix::Multiply,
        BlendMode::Screen => Mix::Screen,
        BlendMode::Overlay => Mix::Overlay,
        BlendMode::Darken => Mix::Darken,
        BlendMode::Lighten => Mix::Lighten,
        BlendMode::ColorDodge => Mix::ColorDodge,
        BlendMode::ColorBurn => Mix::ColorBurn,
        BlendMode::HardLight => Mix::HardLight,
        BlendMode::SoftLight => Mix::SoftLight,
        BlendMode::Difference => Mix::Difference,
        BlendMode::Exclusion => Mix::Exclusion,
        BlendMode::Hue => Mix::Hue,
        BlendMode::Saturation => Mix::Saturation,
        BlendMode::Color => Mix::Color,
        BlendMode::Luminosity => Mix::Luminosity,
    }
}
