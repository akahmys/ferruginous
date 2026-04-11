//! PDF Rendering engine using the Vello scene graph.
use vello::{Scene, peniko::{Brush, Fill, Color as VelloColor, ImageBrush, ImageData, ImageFormat, ImageAlphaType, Blob, BlendMode as VelloBlendMode, Mix}, kurbo::{Affine, Stroke, Rect, Point}};
use ferruginous_sdk::graphics::{DrawOp, Color, ClippingRule, BlendMode};
use rayon::prelude::*;

/// Core PDF renderer using the Vello scene graph.
pub struct Renderer {
    scene: Scene,
}

impl Renderer {
    /// Creates a new Renderer with an empty scene.
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
        }
    }

    /// Clears the current scene.
    pub fn clear(&mut self) {
        self.scene = Scene::new();
    }

    /// Renders a page display list into the Vello scene with a view transform.
    pub fn render_display_list(&mut self, list: &[DrawOp], view_transform: Affine) {
        let mut transform_stack = vec![view_transform];
        let mut layer_stack = Vec::new(); // Tracks how many vello layers to pop per Q

        for op in list {
            match op {
                DrawOp::PushState => {
                    let top = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    transform_stack.push(top);
                    layer_stack.push(0);
                }
                DrawOp::PopState => {
                    if let Some(count) = layer_stack.pop() {
                        for _ in 0..count {
                            self.scene.pop_layer();
                        }
                    }
                    if transform_stack.len() > 1 {
                        transform_stack.pop();
                    }
                }
                DrawOp::SetTransform(affine) => {
                    if let Some(top) = transform_stack.last_mut() {
                        *top = *top * *affine;
                    }
                }
                DrawOp::FillPath { path, color, rule, blend_mode, alpha } => {
                    let brush = to_vello_brush(color);
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    // Use a layer for blend mode/alpha
                    self.scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, transform, path.as_ref());
                    self.scene.fill(to_vello_fill(rule), Affine::IDENTITY, &brush, None, path.as_ref());
                    self.scene.pop_layer();
                }
                DrawOp::StrokePath { path, color, width, blend_mode, alpha } => {
                    let brush = to_vello_brush(color);
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    let stroke = Stroke::new(*width);
                    self.scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, transform, path.as_ref());
                    self.scene.stroke(&stroke, Affine::IDENTITY, &brush, None, path.as_ref());
                    self.scene.pop_layer();
                }
                DrawOp::Clip(path, _rule) => {
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    // Vello 0.8.0 push_layer (clip_style, blend, alpha, transform, clip)
                    self.scene.push_layer(Fill::NonZero, VelloBlendMode::default(), 1.0, transform, path.as_ref());
                    if let Some(count) = layer_stack.last_mut() {
                        *count += 1;
                    }
                }
                DrawOp::DrawText { glyphs, color, blend_mode, alpha, .. } => {
                    let brush = to_vello_brush(color);
                    for glyph in glyphs {
                        
                        if let Some(ref path) = glyph.path {
                            self.scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, Affine::IDENTITY, path.as_ref());
                            self.scene.fill(
                                vello::peniko::Fill::NonZero,
                                Affine::IDENTITY,
                                &brush,
                                None,
                                path.as_ref(),
                            );
                            self.scene.pop_layer();
                        } else {
                            self.scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, Affine::IDENTITY, &glyph.bbox);
                            self.scene.fill(
                                vello::peniko::Fill::NonZero,
                                Affine::IDENTITY,
                                &brush,
                                None,
                                &glyph.bbox,
                            );
                            self.scene.pop_layer();
                        }
                    }
                }
                DrawOp::DrawShading { shading, blend_mode, alpha } => {
                     let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                     let triangles = ferruginous_sdk::shading_tess::tessellate_shading(shading);
                     self.scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, transform, &Rect::from_origin_size(Point::new(-10000.0, -10000.0), (20000.0, 20000.0)));
                     for tri in triangles {
                         let mut path = vello::kurbo::BezPath::new();
                         path.move_to(tri.v[0].point);
                         path.line_to(tri.v[1].point);
                         path.line_to(tri.v[2].point);
                         path.close_path();
                         
                         let color = vello::peniko::Color::new([tri.v[0].color[0], tri.v[0].color[1], tri.v[0].color[2], tri.v[0].color[3]]);
                         self.scene.fill(vello::peniko::Fill::NonZero, Affine::IDENTITY, color, None, &path);
                     }
                     self.scene.pop_layer();
                }
                DrawOp::DrawPath(path, color, width) => {
                    let brush = to_vello_brush(color);
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    let stroke = Stroke::new(*width);
                    self.scene.stroke(&stroke, transform, &brush, None, path.as_ref());
                }
                DrawOp::DrawImage { data, width, height, components, rect, blend_mode, alpha } => {
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    let image = to_vello_image(data, *width, *height, *components);
                    self.scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, transform, rect);
                    self.scene.fill(Fill::NonZero, Affine::IDENTITY, &image, None, rect);
                    self.scene.pop_layer();
                }
            }
        }
    }

    /// Renders selection highlights (translucent rectangles) over the scene.
    pub fn render_highlights(&mut self, rects: &[Rect], view_transform: Affine) {
        let highlight_brush = Brush::Solid(VelloColor::new([0.0, 0.5, 1.0, 0.4])); // Translucent Blue
        for rect in rects {
            // Highlights are already in page space, just apply view_transform (zoom/pan)
            self.scene.fill(Fill::NonZero, view_transform, &highlight_brush, None, rect);
        }
    }

    /// Returns a reference to the current Vello scene.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
}

fn to_vello_brush(color: &Color) -> Brush {
    match color {
        Color::Gray(g) => Brush::Solid(VelloColor::new([*g as f32, *g as f32, *g as f32, 1.0])),
        Color::RGB(r, g, b) => Brush::Solid(VelloColor::new([*r as f32, *g as f32, *b as f32, 1.0])),
        Color::CMYK(c, m, y, k) => {
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);
            Brush::Solid(VelloColor::new([r as f32, g as f32, b as f32, 1.0]))
        }
        _ => Brush::Solid(VelloColor::new([0.0, 0.0, 0.0, 1.0])),
    }
}

fn to_vello_fill(rule: &ClippingRule) -> Fill {
    match rule {
        ClippingRule::NonZeroWinding => Fill::NonZero,
        ClippingRule::EvenOdd => Fill::EvenOdd,
    }
}

fn to_vello_blend(mode: &BlendMode) -> VelloBlendMode {
    VelloBlendMode {
        mix: match mode {
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
        },
        compose: vello::peniko::Compose::SrcOver,
    }
}

fn to_vello_image(data: &[u8], width: u32, height: u32, components: u8) -> ImageBrush {
    let mut rgba_data = vec![0u8; (width * height * 4) as usize];
    match components {
        3 => { // RGB to RGBA
            rgba_data.par_chunks_exact_mut(4).enumerate().for_each(|(i, chunk)| {
                let src_idx = i * 3;
                if src_idx + 2 < data.len() {
                    chunk[0] = data[src_idx];
                    chunk[1] = data[src_idx+1];
                    chunk[2] = data[src_idx+2];
                    chunk[3] = 255;
                }
            });
        }
        1 => { // Gray to RGBA
            rgba_data.par_chunks_exact_mut(4).enumerate().for_each(|(i, chunk)| {
                if i < data.len() {
                    let g = data[i];
                    chunk[0] = g;
                    chunk[1] = g;
                    chunk[2] = g;
                    chunk[3] = 255;
                }
            });
        }
        _ => {
            // Default blank for unsupported color spaces in prototype
            rgba_data.resize((width * height * 4) as usize, 0);
        }
    }
    // In peniko 0.6.0, we use ImageBrush directly from ImageData
    ImageBrush::from(ImageData {
        data: Blob::from(rgba_data),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::AlphaPremultiplied,
        width,
        height,
    })
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
