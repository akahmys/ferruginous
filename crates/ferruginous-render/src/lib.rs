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

use vello::{Scene, peniko::Brush};
use kurbo::{Affine, BezPath, Stroke, Cap, Join};
use ferruginous_core::graphics::{Color, WindingRule, StrokeStyle, LineCap, LineJoin, PixelFormat};

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
}

/// Vello-based implementation of [RenderBackend].
pub struct VelloBackend {
    scene: Scene,
    transform_stack: Vec<Affine>,
    current_transform: Affine,
    fill_alpha: f64,
    stroke_alpha: f64,
    blend_mode: ferruginous_core::graphics::BlendMode,
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
            transform_stack: Vec::new(),
            current_transform: Affine::IDENTITY,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend_mode: ferruginous_core::graphics::BlendMode::Normal,
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }
}

impl RenderBackend for VelloBackend {
    fn push_state(&mut self) {
        self.transform_stack.push(self.current_transform);
    }

    fn pop_state(&mut self) {
        self.current_transform = self.transform_stack.pop().unwrap_or(Affine::IDENTITY);
    }

    fn transform(&mut self, affine: Affine) {
        self.current_transform *= affine;
    }

    fn fill_path(&mut self, path: &BezPath, color: &Color, rule: WindingRule) {
        let alpha = self.fill_alpha as f32;
        let brush = to_vello_brush(color, alpha);
        let fill = match rule {
            WindingRule::NonZero => vello::peniko::Fill::NonZero,
            WindingRule::EvenOdd => vello::peniko::Fill::EvenOdd,
        };
        
        let mix = to_vello_mix(self.blend_mode);
        if mix != vello::peniko::Mix::Normal {
            let blend = vello::peniko::BlendMode::new(mix, vello::peniko::Compose::SrcOver);
            // Vello push_layer takes (clip_style, blend, alpha, transform, clip)
            self.scene.push_layer(vello::peniko::Fill::NonZero, blend, 1.0, self.current_transform, path);
            self.scene.fill(fill, Affine::IDENTITY, &brush, None, path);
            self.scene.pop_layer();
        } else {
            self.scene.fill(fill, self.current_transform, &brush, None, path);
        }
    }

    fn stroke_path(&mut self, path: &BezPath, color: &Color, style: &StrokeStyle) {
        let alpha = self.stroke_alpha as f32;
        let brush = to_vello_brush(color, alpha);
        let stroke = to_kurbo_stroke(style);
        
        let mix = to_vello_mix(self.blend_mode);
        if mix != vello::peniko::Mix::Normal {
            let blend = vello::peniko::BlendMode::new(mix, vello::peniko::Compose::SrcOver);
            self.scene.push_layer(vello::peniko::Fill::NonZero, blend, 1.0, self.current_transform, path);
            if let Some((array, phase)) = &style.dash_pattern {
                let dashed: BezPath = kurbo::dash(path.iter(), *phase, array).collect();
                self.scene.stroke(&stroke, Affine::IDENTITY, &brush, None, &dashed);
            } else {
                self.scene.stroke(&stroke, Affine::IDENTITY, &brush, None, path);
            }
            self.scene.pop_layer();
        } else {
            if let Some((array, phase)) = &style.dash_pattern {
                let dashed: BezPath = kurbo::dash(path.iter(), *phase, array).collect();
                self.scene.stroke(&stroke, self.current_transform, &brush, None, &dashed);
            } else {
                self.scene.stroke(&stroke, self.current_transform, &brush, None, path);
            }
        }
    }

    fn push_clip(&mut self, path: &BezPath, rule: WindingRule) {
        let fill = match rule {
            WindingRule::NonZero => vello::peniko::Fill::NonZero,
            WindingRule::EvenOdd => vello::peniko::Fill::EvenOdd,
        };
        self.scene.push_clip_layer(fill, self.current_transform, path);
    }

    fn pop_clip(&mut self) {
        self.scene.pop_layer();
    }

    fn draw_image(&mut self, data: &[u8], width: u32, height: u32, format: PixelFormat) {
        use vello::peniko::{Blob, ImageFormat, ImageData, ImageAlphaType};
        
        let v_format = match format {
            PixelFormat::Gray8 => ImageFormat::Rgba8, 
            PixelFormat::Rgb8 => ImageFormat::Rgba8, 
            PixelFormat::Cmyk8 => ImageFormat::Rgba8, 
        };

        let data_vec = data.to_vec();
        let blob_data: std::sync::Arc<dyn AsRef<[u8]> + Send + Sync> = std::sync::Arc::new(data_vec);
        let image = ImageData {
            data: Blob::new(blob_data),
            format: v_format,
            alpha_type: ImageAlphaType::Alpha,
            width,
            height,
        };
        
        // Image transparency
        let mix = to_vello_mix(self.blend_mode);
        if mix != vello::peniko::Mix::Normal || self.fill_alpha < 1.0 {
            let blend = vello::peniko::BlendMode::new(mix, vello::peniko::Compose::SrcOver);
            let rect = kurbo::Rect::new(0.0, 0.0, width as f64, height as f64);
            self.scene.push_layer(vello::peniko::Fill::NonZero, blend, self.fill_alpha as f32, self.current_transform, &rect);
            self.scene.draw_image(&image, Affine::IDENTITY);
            self.scene.pop_layer();
        } else {
            self.scene.draw_image(&image, self.current_transform);
        }
    }

    fn set_fill_alpha(&mut self, alpha: f64) {
        self.fill_alpha = alpha;
    }

    fn set_stroke_alpha(&mut self, alpha: f64) {
        self.stroke_alpha = alpha;
    }

    fn set_blend_mode(&mut self, mode: ferruginous_core::graphics::BlendMode) {
        self.blend_mode = mode;
    }
}

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
        Color::Gray(g) => Brush::Solid(vello::peniko::Color::new([*g as f32, *g as f32, *g as f32, alpha])),
        Color::Rgb(r, g, b) => Brush::Solid(vello::peniko::Color::new([*r as f32, *g as f32, *b as f32, alpha])),
        Color::Cmyk(c, m, y, k) => {
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);
            Brush::Solid(vello::peniko::Color::new([r as f32, g as f32, b as f32, alpha]))
        }
    }
}

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
