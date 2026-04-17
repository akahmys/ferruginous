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
use ferruginous_core::graphics::{Color, WindingRule, StrokeStyle, LineCap, LineJoin};

/// Trait defining the core functionality of a rendering backend.
pub trait RenderBackend: Send {
    fn push_state(&mut self);
    fn pop_state(&mut self);
    fn transform(&mut self, affine: Affine);
    
    fn fill_path(&mut self, path: &BezPath, color: &Color, rule: WindingRule);
    fn stroke_path(&mut self, path: &BezPath, color: &Color, style: &StrokeStyle);
    
    fn push_clip(&mut self, path: &BezPath, rule: WindingRule);
    fn pop_clip(&mut self);
}

/// Vello-based implementation of [RenderBackend].
pub struct VelloBackend {
    scene: Scene,
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
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }
}

impl RenderBackend for VelloBackend {
    fn push_state(&mut self) {
    }

    fn pop_state(&mut self) {
    }

    fn transform(&mut self, _affine: Affine) {
    }

    fn fill_path(&mut self, path: &BezPath, color: &Color, rule: WindingRule) {
        let brush = to_vello_brush(color);
        let fill = match rule {
            WindingRule::NonZero => vello::peniko::Fill::NonZero,
            WindingRule::EvenOdd => vello::peniko::Fill::EvenOdd,
        };
        self.scene.fill(fill, Affine::IDENTITY, &brush, None, path);
    }

    fn stroke_path(&mut self, path: &BezPath, color: &Color, style: &StrokeStyle) {
        let brush = to_vello_brush(color);
        let stroke = to_kurbo_stroke(style);
        
        if let Some((array, phase)) = &style.dash_pattern {
            // Apply dash pattern to path elements
            let dashed: BezPath = kurbo::dash(path.iter(), *phase, array).collect();
            self.scene.stroke(&stroke, Affine::IDENTITY, &brush, None, &dashed);
        } else {
            self.scene.stroke(&stroke, Affine::IDENTITY, &brush, None, path);
        }
    }

    fn push_clip(&mut self, path: &BezPath, rule: WindingRule) {
        let fill = match rule {
            WindingRule::NonZero => vello::peniko::Fill::NonZero,
            WindingRule::EvenOdd => vello::peniko::Fill::EvenOdd,
        };
        self.scene.push_clip_layer(fill, Affine::IDENTITY, path);
    }

    fn pop_clip(&mut self) {
        self.scene.pop_layer();
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

fn to_vello_brush(color: &Color) -> vello::peniko::Brush {
    match color {
        Color::Gray(g) => Brush::Solid(vello::peniko::Color::new([*g as f32, *g as f32, *g as f32, 1.0])),
        Color::Rgb(r, g, b) => Brush::Solid(vello::peniko::Color::new([*r as f32, *g as f32, *b as f32, 1.0])),
        Color::Cmyk(c, m, y, k) => {
            // Baseline conversion
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);
            Brush::Solid(vello::peniko::Color::new([r as f32, g as f32, b as f32, 1.0]))
        }
    }
}
