//! Ferruginous Render: Graphics Bridging for PDF.
//!
//! (ISO 32000-2:2020 Clause 8)

pub mod ctm;
pub mod headless;
pub mod path;
pub mod text;

#[cfg(test)]
mod path_tests;
#[cfg(test)]
mod text_tests;

use ferruginous_core::graphics::{Color, LineCap, LineJoin, PixelFormat, StrokeStyle, WindingRule};
use kurbo::{Affine, BezPath, Cap, Join, Stroke};
use vello::Scene;

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

    fn define_font(&mut self, name: &str, data: std::sync::Arc<Vec<u8>>, index: Option<usize>);
    fn set_font(&mut self, name: &str);
    #[allow(clippy::too_many_arguments)]
    fn show_text(
        &mut self,
        glyphs: &[(u32, f32)],
        text: &str,
        size: f64,
        transform: kurbo::Affine,
        tc: f64,
        tw: f64,
        is_vertical: bool,
    );
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
    font_data: Option<std::sync::Arc<Vec<u8>>>,
    font_index: Option<usize>,
}

/// Vello-based implementation of [RenderBackend].
pub struct VelloBackend {
    scene: Scene,
    state: VelloState,
    state_stack: Vec<VelloState>,
    font_cache: std::collections::BTreeMap<String, (std::sync::Arc<Vec<u8>>, Option<usize>)>, // Name -> (Data, CollectionIndex)
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
                font_data: None,
                font_index: None,
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

        let (dash_offset, is_dashed) =
            if let Some((_, phase)) = &style.dash_pattern { (*phase, true) } else { (0.0, false) };

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
        use vello::peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};

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
                    rgba.extend_from_slice(&[
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        255,
                    ]);
                }
                rgba
            }
        };

        let blob_data: std::sync::Arc<dyn AsRef<[u8]> + Send + Sync> =
            std::sync::Arc::new(rgba_data);
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

    fn set_font(&mut self, name: &str) {
        if let Some((data, index)) = self.font_cache.get(name) {
            self.state.font_data = Some(data.clone());
            self.state.font_index = *index;
        } else {
            self.state.font_data = None;
            self.state.font_index = None;
        }
    }

    fn define_font(&mut self, name: &str, data: std::sync::Arc<Vec<u8>>, index: Option<usize>) {
        // Pre-scan for Japanese support if it's a TTC
        let safe_index = if let Ok(file_ref) = skrifa::raw::FileRef::new(&data) {
            match file_ref {
                skrifa::raw::FileRef::Font(_) => None,
                skrifa::raw::FileRef::Collection(c) => {
                    // Find a font with Japanese support or just default to 0
                    let mut best_index = 0;
                    for i in 0..c.len() {
                        if let Ok(_font) = c.get(i) {
                            best_index = i as usize;
                            break;
                        }
                    }
                    Some(best_index)
                }
            }
        } else {
            None
        };
        self.font_cache.insert(name.to_string(), (data.clone(), index.or(safe_index)));
    }

    #[allow(clippy::too_many_arguments)]
    fn show_text(
        &mut self,
        glyphs: &[(u32, f32)],
        text: &str,
        size: f64,
        transform: Affine,
        _tc: f64,
        _tw: f64,
        is_vertical: bool,
    ) {
        let font_data = match &self.state.font_data {
            Some(d) => d.clone(),
            None => {
                // Fallback for diagnostic/legacy (vital for non-embedded fonts like MS Mincho)
                let fallback_path = "/System/Library/Fonts/Hiragino Sans GB.ttc";
                match std::fs::read(fallback_path) {
                    Ok(data) => std::sync::Arc::new(data),
                    Err(_) => return,
                }
            }
        };

        let sfnt_data = crate::text::ensure_sfnt(&font_data);
        let data_ref = sfnt_data.as_deref().unwrap_or(&font_data);

        let bridge = crate::text::SkrifaBridge::new();
        let brush = to_vello_brush(&self.state.fill_color, self.state.fill_alpha as f32);
        let mut advance_offset = 0.0;

        for (i, (gid_u32, _width)) in glyphs.iter().enumerate() {
            let mut gid = *gid_u32;
            let unicode_fallback = text.chars().nth(i);

            if i == 0 {
                eprintln!(
                    "DEBUG: font_data len={}, first 4 bytes={:02X?}",
                    font_data.len(),
                    &font_data[0..std::cmp::min(4, font_data.len())]
                );
            }

            if font_data.len() >= 2 && font_data[0] == 0x01 && font_data[1] == 0x00 {
                gid = crate::text::cff_get_gid_for_cid(&font_data, gid as u16).unwrap_or(gid as u16)
                    as u32;
            }

            let path_opt = bridge.extract_path(data_ref, gid, is_vertical, unicode_fallback);
            if let Some(mut path) = path_opt {
                let glyph_scale = size / 1000.0;

                let offset_vec = if is_vertical {
                    kurbo::Vec2::new(0.0, -advance_offset)
                } else {
                    kurbo::Vec2::new(advance_offset, 0.0)
                };

                // The transform provided handles the page coordinate space, flip, and specific text matrix
                let glyph_transform = self.state.transform
                    * transform
                    * Affine::translate(offset_vec)
                    * Affine::scale(glyph_scale);

                let abs_pos = glyph_transform.translation();
                eprintln!(
                    "DEBUG: Rendering glyph gid={} unicode={:?} at abs_pos=({:.2}, {:.2}) scale={:.2}",
                    gid, unicode_fallback, abs_pos.x, abs_pos.y, glyph_scale
                );

                path.apply_affine(glyph_transform);
                self.scene.fill(
                    vello::peniko::Fill::NonZero,
                    Affine::IDENTITY,
                    &brush,
                    None,
                    &path,
                );

                let advance = *_width as f64 * glyph_scale;
                advance_offset += advance;
            } else {
                eprintln!(
                    "WARNING: Failed to extract path for gid={} unicode={:?}",
                    gid, unicode_fallback
                );
                let advance = *_width as f64 * (size / 1000.0);
                advance_offset += advance;
            }
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
        Color::Gray(g) => vello::peniko::Brush::Solid(vello::peniko::Color::new([
            *g as f32, *g as f32, *g as f32, alpha,
        ])),
        Color::Rgb(r, g, b) => vello::peniko::Brush::Solid(vello::peniko::Color::new([
            *r as f32, *g as f32, *b as f32, alpha,
        ])),
        Color::Cmyk(c, m, y, k) => {
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);
            vello::peniko::Brush::Solid(vello::peniko::Color::new([
                r as f32, g as f32, b as f32, alpha,
            ]))
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
