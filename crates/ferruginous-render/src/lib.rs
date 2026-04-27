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
use skrifa::raw::FileRef;
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

    fn define_font(
        &mut self,
        name: &str,
        base_name: Option<&str>,
        data: Option<std::sync::Arc<Vec<u8>>>,
        index: Option<usize>,
        cid_to_gid_map: Option<Vec<u16>>,
    );
    fn set_font(&mut self, name: &str);
    #[allow(clippy::too_many_arguments)]
    fn show_text(
        &mut self,
        glyphs: &[(u32, f32, f32, f32, u32)], // (cid, advance, vx, vy, char_code)
        text: &str,
        size: f64,
        transform: kurbo::Affine,
        tc: f64,
        tw: f64,
        is_vertical: bool,
        op_index: usize,
    );
    fn set_text_render_mode(&mut self, mode: ferruginous_core::graphics::TextRenderingMode);
    fn set_char_spacing(&mut self, spacing: f64);
    fn set_word_spacing(&mut self, spacing: f64);
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
    cid_to_gid_map: Option<Vec<u16>>,
    text_render_mode: i32,
    char_spacing: f64,
    word_spacing: f64,
    font_name: Option<String>,
}

/// Cached font information to satisfy clippy complexity rules.
struct FontCacheEntry {
    data: Option<std::sync::Arc<Vec<u8>>>,
    sfnt_data: Option<std::sync::Arc<Vec<u8>>>,
    collection_index: Option<usize>,
    cid_to_gid_map: Option<Vec<u16>>,
    base_name: Option<String>,
}

/// Vello-based implementation of [RenderBackend].
pub struct VelloBackend {
    scene: Scene,
    state: VelloState,
    state_stack: Vec<VelloState>,
    font_cache: std::collections::BTreeMap<String, FontCacheEntry>,
    pub system_fonts: std::collections::BTreeMap<String, Vec<u8>>,
    skrifa_bridge: crate::text::SkrifaBridge,
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
                cid_to_gid_map: None,
                text_render_mode: 0,
                char_spacing: 0.0,
                word_spacing: 0.0,
                font_name: None,
            },
            state_stack: Vec::new(),
            font_cache: std::collections::BTreeMap::new(),
            system_fonts: {
                let mut fonts = std::collections::BTreeMap::new();
                let font_path = "/System/Library/Fonts/ヒラギノ明朝 ProN.ttc";
                if let Ok(data) = std::fs::read(font_path) {
                    fonts.insert("ヒラギノ明朝 ProN".to_string(), data);
                }
                fonts
            },
            skrifa_bridge: crate::text::SkrifaBridge::new(None),
        }
    }

    pub fn set_transform(&mut self, transform: Affine) {
        self.state.transform = transform;
    }

    pub fn define_font(
        &mut self,
        name: &str,
        data: Option<&std::sync::Arc<Vec<u8>>>,
        index: usize,
        cid_to_gid_map: Option<&[u16]>,
        base_name: Option<&str>,
    ) {
        eprintln!("fepdf: define_font name={} data_present={} base_name={:?}", name, data.is_some(), base_name);
        self.font_cache.insert(
            name.to_string(),
            FontCacheEntry {
                data: data.cloned(),
                sfnt_data: None,
                collection_index: Some(index),
                cid_to_gid_map: cid_to_gid_map.map(|m| m.to_vec()),
                base_name: base_name.map(|s| s.to_string()),
            },
        );
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
        if let Some(old_state) = self.state_stack.pop() {
            while self.state.clip_count > old_state.clip_count {
                self.pop_clip();
            }
            self.state = old_state;
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

    fn define_font(
        &mut self,
        name: &str,
        base_name: Option<&str>,
        data: Option<std::sync::Arc<Vec<u8>>>,
        index: Option<usize>,
        cid_to_gid_map: Option<Vec<u16>>,
    ) {
        // Resolve font index if not provided (for collections)
        let resolved_index = if index.is_none() {
            if let Some(data_arc) = data.as_ref() {
                if let Ok(file_ref) = skrifa::raw::FileRef::new(data_arc) {
                    match file_ref {
                        skrifa::raw::FileRef::Font(_) => None,
                        skrifa::raw::FileRef::Collection(c) => {
                            let mut best = 0;
                            for i in 0..c.len() {
                                if c.get(i).is_ok() {
                                    best = i;
                                    break;
                                }
                            }
                            Some(best as usize)
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            index
        };

        self.font_cache.insert(
            name.to_string(),
            FontCacheEntry {
                data,
                sfnt_data: None,
                collection_index: resolved_index,
                cid_to_gid_map,
                base_name: base_name.map(|s| s.to_string()),
            },
        );
    }

    fn set_font(&mut self, name: &str) {
        if let Some(entry) = self.font_cache.get(name) {
            self.state.font_data = entry.data.clone();
            self.state.font_index = entry.collection_index;
            self.state.cid_to_gid_map = entry.cid_to_gid_map.clone();
            self.state.font_name = entry.base_name.clone();
        } else {
            self.state.font_data = None;
            self.state.font_index = None;
            self.state.cid_to_gid_map = None;
            self.state.font_name = None;
        }
    }

    fn set_text_render_mode(&mut self, mode: ferruginous_core::graphics::TextRenderingMode) {
        self.state.text_render_mode = mode as i32;
    }

    fn set_char_spacing(&mut self, spacing: f64) {
        self.state.char_spacing = spacing;
    }

    fn set_word_spacing(&mut self, spacing: f64) {
        self.state.word_spacing = spacing;
    }

    #[allow(clippy::too_many_arguments)]
    fn show_text(
        &mut self,
        glyphs: &[(u32, f32, f32, f32, u32)],
        text: &str,
        size: f64,
        transform: Affine,
        _tc: f64,
        _tw: f64,
        is_vertical: bool,
        _op_index: usize,
    ) {
        let font_data_arc = self.state.font_data.clone();
        let font_data_raw = font_data_arc.as_deref().map(|d| d.as_slice()).unwrap_or(&[]);
        
        let (data_ref, _sfnt_holder): (&[u8], Option<std::sync::Arc<Vec<u8>>>) = 
        if let Some(font_name) = &self.state.font_name {
            if let Some(entry) = self.font_cache.get_mut(font_name) {
                if entry.sfnt_data.is_none() {
                    let raw = entry.data.as_deref().map(|d| d.as_slice()).unwrap_or(&[]);
                    if let Some(sfnt) = crate::text::ensure_sfnt(raw) {
                        entry.sfnt_data = Some(std::sync::Arc::new(sfnt));
                    }
                }
                if let Some(sfnt_arc) = &entry.sfnt_data {
                    (sfnt_arc.as_slice(), Some(sfnt_arc.clone()))
                } else {
                    let raw = entry.data.as_deref().map(|d| d.as_slice()).unwrap_or(&[]);
                    (raw, entry.data.clone())
                }
            } else {
                (font_data_raw, font_data_arc.clone())
            }
        } else {
            (font_data_raw, font_data_arc.clone())
        };
        let data_ref: &[u8] = data_ref;

        let start_time = std::time::Instant::now();
        let mut glyph_count = 0;
        let _cache_hits = 0;

        if self.skrifa_bridge.primary_system_font.is_none()
            && let Some(f) = self.system_fonts.get("ヒラギノ明朝 ProN") {
            self.skrifa_bridge.primary_system_font = Some(f.clone());
        }

        let brush = to_vello_brush(&self.state.fill_color, self.state.fill_alpha as f32);
        let mut advance_offset = 0.0;

        let cid_to_gid_map = self.state.cid_to_gid_map.as_deref();
        let font_name = self.state.font_name.as_ref();
        let is_japanese = font_name
            .map(|n| {
                let nl = n.to_lowercase();
                nl.contains("hira") || nl.contains("mincho") || nl.contains("gothic") || nl.contains("koz") || nl.contains("aj1") || nl.contains("ipa") || nl.contains("ms-")
            })
            .unwrap_or(false);

        // Pre-parse system font once for this show_text operation to avoid heavy overhead in the loop
        let system_font_data = self.system_fonts.get("ヒラギノ明朝 ProN").map(|v| v.as_slice());
        let system_font_file = system_font_data.and_then(|d| FileRef::new(d).ok());
        let system_font_ref = system_font_file.and_then(|f| match f {
            FileRef::Font(font) => Some(font),
            FileRef::Collection(c) => c.get(0).ok(),
        });

        // Pre-parse primary font once for this show_text operation
        let primary_font_file = FileRef::new(data_ref).ok();
        let primary_font_ref = primary_font_file.and_then(|f| match f {
            FileRef::Font(font) => Some(font),
            FileRef::Collection(c) => c.get(self.state.font_index.unwrap_or(0) as u32).ok(),
        });

        let mut char_iter = text.chars();
        for (gid_u32, _width, vx, vy, char_code) in glyphs.iter() {
            let gid = *gid_u32;
            let char_code = *char_code;
            let unicode_fallback = char_iter.next();

            // Skip rendering for common whitespace or if mode is 3 (Invisible)
            let is_whitespace_code =
                char_code == 0x20 || char_code == 0x09 || char_code == 0x0A || char_code == 0x0D;

            let is_repurposed_japanese = char_code == 0x20
                && unicode_fallback.map(|c| (c as u32) >= 0x2E80).unwrap_or(false);
            let should_skip =
                self.state.text_render_mode == 3 || (is_whitespace_code && !is_repurposed_japanese);
            if should_skip {
                let glyph_scale = size / 1000.0;
                let advance = *_width as f64 * glyph_scale;
                advance_offset += advance;
                continue;
            }

            let path_opt: Option<BezPath> = self.skrifa_bridge.extract_path(
                data_ref,
                gid,
                char_code,
                cid_to_gid_map,
                is_vertical,
                unicode_fallback,
                is_japanese,
                false, // force_system_fallback
                system_font_ref.as_ref(),
                primary_font_ref.as_ref(),
            );

            glyph_count += 1;
            if path_opt.is_some() {
                // Total glyphs processed.
            }

            if let Some(path) = path_opt {
                // Get units_per_em from the font if possible, fallback to 1000
                let units_per_em = self.skrifa_bridge.get_units_per_em(data_ref).unwrap_or(1000) as f64;
                let glyph_scale = size / units_per_em;
                let metrics_scale = size / 1000.0;
                let origin_shift = kurbo::Vec2::new(-*vx as f64, -*vy as f64);
                let writing_line_advance = if is_vertical {
                    Affine::translate((0.0, -advance_offset))
                } else {
                    Affine::translate((advance_offset, 0.0))
                };

                let glyph_transform = self.state.transform
                    * transform
                    * writing_line_advance
                    * Affine::scale(glyph_scale)
                    * Affine::translate(origin_shift);

                let mut path: kurbo::BezPath = path;
                path.apply_affine(glyph_transform);
                self.scene.fill(
                    vello::peniko::Fill::NonZero,
                    Affine::IDENTITY,
                    &brush,
                    None,
                    &path,
                );

                let advance = *_width as f64 * metrics_scale;
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

        let elapsed = start_time.elapsed();
        if elapsed.as_millis() > 10 {
            eprintln!(
                "show_text: processed {} glyphs in {:?} (font: {:?})",
                glyph_count,
                elapsed,
                self.state.font_name.as_deref().unwrap_or("unknown")
            );
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
