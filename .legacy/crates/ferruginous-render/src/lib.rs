//! Ferruginous Rendering Engine
//! 
//! This crate provides an abstraction layer and implementation for bridging
//! PDF drawing commands (`DrawOp`) to a specific graphics library (currently Vello).

use vello::{Scene, peniko::{Brush, Fill, Color as VelloColor, ImageBrush, ImageData, ImageFormat, ImageAlphaType, Blob, BlendMode as VelloBlendMode, Mix, ColorStop, Gradient}, kurbo::{self, Affine, Stroke, Rect, Shape, BezPath, Point as KurboPoint}};
use ferruginous_sdk::graphics::{DrawOp, DrawCommand, Color, ClippingRule, BlendMode};
// use ferruginous_sdk::core::Object; // Removed unused import
use ferruginous_sdk::ocg::OCContext;
use rayon::prelude::*;
use std::sync::Arc;

/// Test harness for performning headless GPU rendering and image capture.
pub mod visual_harness;

/// Options for the rendering engine.
#[derive(Debug, Clone, Default)]
pub struct BackendOptions {
    /// Whether to use CPU rendering.
    pub use_cpu: bool,
    /// Whether to enable anti-aliasing.
    pub antialiasing: bool,
}

/// Trait defining the core functionality of a rendering backend.
pub trait RenderBackend: Send {
    /// Initializes the rendering state (e.g., scenes).
    fn clear(&mut self);
    
    /// Renders the specified display list by applying a view transformation.
    fn render_display_list(&mut self, list: &[DrawCommand], transform: Affine, oc_context: Option<&OCContext>);
    
    /// Renders highlights (e.g., selection ranges) on top of the scene.
    fn render_highlights(&mut self, rects: &[Rect], view_transform: Affine);

    /// Prepares (initializes) the GPU renderer.
    fn prepare_renderer(&mut self, device: &wgpu::Device, options: BackendOptions) -> Result<(), String>;

    /// Renders the current scene to the specified texture target.
    fn render_to_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) -> Result<(), String>;
}

/// `RenderBackend` implementation using the Vello graphics engine.
pub struct VelloBackend {
    /// Vello scene graph accumulating drawing content.
    scene: Scene,
    /// Vello renderer responsible for actual GPU rendering.
    renderer: Option<vello::Renderer>,
}

impl VelloBackend {
    /// Creates a new `VelloBackend` with an empty display list and initial state.
    #[must_use] 
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            renderer: None,
        }
    }

    /// Returns a reference to the internally managed Vello `Scene`.
    #[must_use] 
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Renders a page display list into the provided Vello scene.
    pub fn render_display_list_to_scene(list: &[DrawCommand], view_transform: Affine, oc_context: Option<&OCContext>, scene: &mut Scene) {
        let mut transform_stack = vec![view_transform];
        let mut layer_stack = vec![0]; // Depth tracker for q/Q logic
        let mut total_layers = 0;

        for (cmd_count, cmd) in list.iter().enumerate() {
            if let (Some(ctx), Some(oc_ref)) = (oc_context, cmd.oc)
                && !*ctx.states.get(&oc_ref).unwrap_or(&true) {
                    continue;
                }
            
            if cmd_count % 100 == 0 {
                // Periodically check that we aren't exploding
                if total_layers > 1000 {
                    eprintln!("[ERROR][RENDER] Layer stack depth exceeded safety limit!");
                    break;
                }
            }
            
            match &cmd.op {
                DrawOp::PushState => {
                    let top = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    transform_stack.push(top);
                    layer_stack.push(0);
                }
                DrawOp::PopState => {
                    if let Some(count) = layer_stack.pop() {
                        for _ in 0..count {
                            if total_layers > 0 {
                                scene.pop_layer();
                                total_layers -= 1;
                            }
                        }
                    }
                    if transform_stack.len() > 1 {
                        transform_stack.pop();
                    }
                }
                DrawOp::SetTransform(affine) => {
                    if !affine.is_finite() {
                        eprintln!("[WARN][RENDER] Skipping non-finite transform: {:?}", affine);
                        continue;
                    }
                    if let Some(top) = transform_stack.last_mut() {
                        *top *= *affine;
                    }
                }
                DrawOp::Clip(path, rule) => {
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    if !transform.is_finite() { continue; }
                    
                    let transformed_path = transform * path.as_ref();
                    let vello_fill = to_vello_fill(rule);
                    
                    // Always use IDENTITY for the layer itself to prevent transform accumulation.
                    scene.push_layer(vello_fill, VelloBlendMode::default(), 1.0, Affine::IDENTITY, &transformed_path);
                    total_layers += 1;
                    if let Some(count) = layer_stack.last_mut() {
                        *count += 1;
                    }
                }
                DrawOp::FillPath { path, color, rule, blend_mode, alpha } => {
                    let brush = to_vello_brush(color);
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    if !transform.is_finite() { continue; }
                    
                    if *alpha < 1.0 || *blend_mode != BlendMode::Normal {
                        // Correctly apply CURRENT transform to calculate the screen-space bbox
                        let path_bbox = path.as_ref().bounding_box();
                        if !path_bbox.is_finite() { continue; }
                        
                        let bbox = transform.transform_rect_bbox(path_bbox);
                        scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, Affine::IDENTITY, &bbox);
                        scene.fill(to_vello_fill(rule), transform, &brush, None, path.as_ref());
                        scene.pop_layer();
                    } else {
                        // HEURISTIC: Skip extremely large black fills (potential background incorrectly defaulted to black)
                        let path_bbox = path.as_ref().bounding_box();
                        if !path_bbox.is_finite() { continue; }

                        let is_very_large = path_bbox.width() > 500.0 && path_bbox.height() > 700.0;
                        let is_black = match color {
                            ferruginous_sdk::graphics::Color::Gray(g) => *g == 0.0,
                            ferruginous_sdk::graphics::Color::RGB(r, g, b) => *r == 0.0 && *g == 0.0 && *b == 0.0,
                            ferruginous_sdk::graphics::Color::CMYK(c, m, y, k) => *k == 1.0 || (*c == 1.0 && *m == 1.0 && *y == 1.0 && *k == 1.0),
                            _ => false,
                        };
                        
                        if is_very_large && is_black {
                            eprintln!("[INFO][RENDER] Suppressing large black fill: {:?}", path_bbox);
                        } else {
                            scene.fill(to_vello_fill(rule), transform, &brush, None, path.as_ref());
                        }
                    }
                }
                DrawOp::StrokePath { 
                    path, color, width, line_cap, line_join, miter_limit, dash_pattern, blend_mode, alpha 
                } => {
                    let brush = to_vello_brush(color);
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    
                    let cap = match line_cap {
                        0 => kurbo::Cap::Butt,
                        1 => kurbo::Cap::Round,
                        2 => kurbo::Cap::Square,
                        _ => kurbo::Cap::Butt,
                    };
                    let join = match line_join {
                        0 => kurbo::Join::Miter,
                        1 => kurbo::Join::Round,
                        2 => kurbo::Join::Bevel,
                        _ => kurbo::Join::Miter,
                    };
                    
                    let stroke = Stroke::new(*width).with_caps(cap).with_join(join).with_miter_limit(*miter_limit);
                    
                    let final_path = if dash_pattern.0.is_empty() {
                        Arc::clone(path)
                    } else {
                        let mut dashed = BezPath::new();
                        for el in kurbo::dash(path.path_elements(0.1), dash_pattern.1, &dash_pattern.0) {
                            dashed.push(el);
                        }
                        Arc::new(dashed)
                    };

                    if *alpha < 1.0 || *blend_mode != BlendMode::Normal {
                        let bbox = transform.transform_rect_bbox(final_path.as_ref().bounding_box());
                        scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, Affine::IDENTITY, &bbox);
                        scene.stroke(&stroke, transform, &brush, None, final_path.as_ref());
                        scene.pop_layer();
                    } else {
                        scene.stroke(&stroke, transform, &brush, None, final_path.as_ref());
                    }
                }
                DrawOp::DrawText { glyphs, color, blend_mode, alpha, .. } => {
                    let brush = to_vello_brush(color);
                    let current_transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    if !current_transform.is_finite() || !view_transform.is_finite() { continue; }
                    
                    let needs_layer = *alpha < 1.0 || *blend_mode != BlendMode::Normal;
                    if needs_layer {
                        let mut page_bbox = Rect::ZERO;
                        for g in glyphs {
                            if g.bbox.is_finite() {
                                page_bbox = page_bbox.union(g.bbox);
                            }
                        }
                        
                        if !page_bbox.is_finite() { continue; }
                        
                        // Only apply clipping layer if transparency or blending REQUIRES it
                        let screen_bbox = view_transform.transform_rect_bbox(page_bbox);
                        scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, Affine::IDENTITY, &screen_bbox);
                    }

                    for glyph in glyphs {
                        if let Some(ref path) = glyph.path {
                            if glyph.point.x.is_finite() && glyph.point.y.is_finite() && !path.as_ref().elements().is_empty() {
                                scene.fill(Fill::NonZero, current_transform, &brush, None, path.as_ref());
                            }
                        }
                    }

                    if needs_layer {
                        scene.pop_layer();
                    }
                }
                DrawOp::DrawImage { data, width, height, components, rect, blend_mode, alpha } => {
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    let _image = to_vello_image(data, *width, *height, *components);
                    if *alpha < 1.0 || *blend_mode != BlendMode::Normal {
                        scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, Affine::IDENTITY, rect);
                        scene.fill(Fill::NonZero, transform, &_image, None, rect);
                        scene.pop_layer();
                    } else {
                        scene.fill(Fill::NonZero, transform, &_image, None, rect);
                    }
                }
                DrawOp::DrawShading { shading, blend_mode, alpha } => {
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    
                    if shading.shading_type == ferruginous_sdk::graphics::ShadingType::Axial && shading.coords.len() == 4 {
                        let p0 = KurboPoint::new(shading.coords[0], shading.coords[1]);
                        let p1 = KurboPoint::new(shading.coords[2], shading.coords[3]);
                        
                        let func_objs = &shading.function;
                        if *alpha < 1.0 || *blend_mode != BlendMode::Normal {
                            let bbox = shading.bbox.unwrap_or(Rect::new(-10000.0, -10000.0, 10000.0, 10000.0));
                            scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, transform, &bbox);
                        }
                        
                        if !func_objs.is_empty() {
                            let stops: Vec<vello::peniko::ColorStop> = if func_objs.len() == 1 {
                                vec![
                                    ColorStop { offset: 0.0, color: VelloColor::new([1.0, 0.0, 0.0, 1.0]).into() },
                                    ColorStop { offset: 1.0, color: VelloColor::new([0.0, 0.0, 1.0, 1.0]).into() },
                                ]
                            } else {
                                func_objs.iter().enumerate().map(|(i, _)| {
                                    let offset = i as f32 / (func_objs.len() as f32 - 1.0);
                                    let color: vello::peniko::Color = if i % 2 == 0 { VelloColor::new([1.0, 0.0, 0.0, 1.0]) } else { VelloColor::new([0.0, 0.0, 1.0, 1.0]) };
                                    ColorStop { offset, color: color.into() }
                                }).collect()
                            };
                            
                            let grad = Gradient::new_linear(p0, p1).with_stops(stops.as_slice());
                            let brush = Brush::Gradient(grad);
                            let fill_rect = shading.bbox.unwrap_or(Rect::new(-10000.0, -10000.0, 10000.0, 10000.0));
                            scene.fill(Fill::NonZero, transform, &brush, None, &fill_rect);
                        }
                    } else {
                        // Fallback to tessellation for mesh types or complex shadings
                        let triangles = ferruginous_sdk::shading_tess::tessellate_shading(shading);
                        if *alpha < 1.0 || *blend_mode != BlendMode::Normal {
                            let bbox = shading.bbox.unwrap_or(Rect::new(-10000.0, -10000.0, 10000.0, 10000.0));
                            scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, transform, &bbox);
                        }
                        for tri in triangles {
                            let mut path = BezPath::new();
                            path.move_to(tri.v[0].point);
                            path.line_to(tri.v[1].point);
                            path.line_to(tri.v[2].point);
                            path.close_path();
                            let color = VelloColor::new([tri.v[0].color[0], tri.v[0].color[1], tri.v[0].color[2], tri.v[0].color[3]]);
                            scene.fill(Fill::NonZero, transform, color, None, &path);
                        }
                    }
                    scene.pop_layer();
                }
                DrawOp::DrawPath(path, color, width) => {
                    let brush = to_vello_brush(color);
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    let stroke = Stroke::new(*width);
                    scene.stroke(&stroke, transform, &brush, None, path.as_ref());
                }
                DrawOp::PushLayer { attrs: _, blend_mode, alpha } => {
                    let transform = *transform_stack.last().unwrap_or(&Affine::IDENTITY);
                    // Use a reasonable bounding rect if possible, otherwise clip to document bounds
                    let clip_rect = Rect::new(-2000.0, -2000.0, 4000.0, 4000.0);
                    scene.push_layer(Fill::NonZero, to_vello_blend(blend_mode), *alpha, transform, &clip_rect);
                    total_layers += 1;
                    layer_stack.push(0); 
                }
                DrawOp::PopLayer => {
                    // Pop all clipping layers within the group first
                    if let Some(count) = layer_stack.pop() {
                        for _ in 0..count {
                            if total_layers > 0 {
                                scene.pop_layer();
                                total_layers -= 1;
                            }
                        }
                    }
                    // Pop the transparency group layer itself
                    if total_layers > 0 {
                        scene.pop_layer();
                        total_layers -= 1;
                    }
                }
            }
        }

        // Final cleanup for un-popped layers
        let root_layers = layer_stack[0];
        for _ in 0..root_layers {
            scene.pop_layer();
        }
    }
}

impl RenderBackend for VelloBackend {
    fn clear(&mut self) {
        self.scene = Scene::new();
    }

    fn render_display_list(&mut self, list: &[DrawCommand], transform: Affine, oc_context: Option<&OCContext>) {
        Self::render_display_list_to_scene(list, transform, oc_context, &mut self.scene);
    }

    fn render_highlights(&mut self, rects: &[Rect], view_transform: Affine) {
        let highlight_brush = Brush::Solid(VelloColor::new([0.0, 0.5, 1.0, 0.4]));
        for rect in rects {
            self.scene.fill(Fill::NonZero, view_transform, &highlight_brush, None, rect);
        }
    }

    fn prepare_renderer(&mut self, device: &wgpu::Device, options: BackendOptions) -> Result<(), String> {
        let vello_options = vello::RendererOptions {
            pipeline_cache: None,
            antialiasing_support: if options.antialiasing { vello::AaSupport::all() } else { vello::AaSupport::area_only() },
            num_init_threads: None,
            use_cpu: options.use_cpu,
        };
        self.renderer = Some(vello::Renderer::new(device, vello_options).map_err(|e| format!("{e:?}"))?);
        Ok(())
    }

    fn render_to_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if let Some(renderer) = &mut self.renderer {
            renderer.render_to_texture(
                device,
                queue,
                &self.scene,
                target_view,
                &vello::RenderParams {
                    base_color: vello::peniko::Color::WHITE,
                    width,
                    height,
                    antialiasing_method: vello::AaConfig::Area,
                },
            ).map_err(|e| format!("{e:?}"))
        } else {
            Err("Renderer not initialized".to_string())
        }
    }
}

#[allow(clippy::many_single_char_names)]
fn to_vello_brush(color: &Color) -> Brush {
    match color {
        Color::Gray(g) => Brush::Solid(VelloColor::new([(*g), (*g), (*g), 1.0])),
        Color::RGB(r, g, b) => Brush::Solid(VelloColor::new([(*r), (*g), (*b), 1.0])),
        Color::CMYK(c, m, y, k) => {
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);
            Brush::Solid(VelloColor::new([r, g, b, 1.0]))
        }
        _ => Brush::Solid(VelloColor::new([0.0, 0.0, 0.0, 1.0])),
    }
}

const fn to_vello_fill(rule: &ClippingRule) -> Fill {
    match rule {
        ClippingRule::NonZeroWinding => Fill::NonZero,
        ClippingRule::EvenOdd => Fill::EvenOdd,
    }
}

const fn to_vello_blend(mode: &BlendMode) -> VelloBlendMode {
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
        3 => {
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
        1 => {
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
            rgba_data.resize((width * height * 4) as usize, 0);
        }
    }
    ImageBrush::from(ImageData {
        data: Blob::from(rgba_data),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::AlphaPremultiplied,
        width,
        height,
    })
}

impl Default for VelloBackend {
    fn default() -> Self {
        Self::new()
    }
}
