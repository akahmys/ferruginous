use egui_wgpu::RenderState;
use std::collections::BTreeMap;
use std::sync::Arc;
use vello::wgpu;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};

struct CachedTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    egui_texture: egui::TextureId,
    width: u32,
    height: u32,
    scene_ptr: Option<*const Scene>,
    last_frame_used: u64,
}

pub struct VelloRenderer {
    renderer: Renderer,
    whole_page_textures: BTreeMap<usize, CachedTexture>,
    tile_textures: BTreeMap<(usize, usize, usize), CachedTexture>, // (page_index, col, row)
    current_frame: u64,
}

impl VelloRenderer {
    pub fn new(device: &wgpu::Device) -> Option<Self> {
        let renderer = Renderer::new(
            device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        )
        .ok()?;

        Some(Self {
            renderer,
            whole_page_textures: BTreeMap::new(),
            tile_textures: BTreeMap::new(),
            current_frame: 0,
        })
    }

    /// Increments the frame counter for texture garbage collection.
    pub fn next_frame(&mut self, render_state: &RenderState) {
        self.current_frame += 1;
        self.gc_textures(render_state);
    }

    /// Renders a page using viewport virtualization and adaptive tiling.
    /// Returns a list of textures and screen-space rects to draw.
    pub fn render_page_virtual( // RR-15 Limit: GUI - Performs sequential declarative layout rendering for vello graphics and virtual tiles
        &mut self,
        render_state: &RenderState,
        scene: &Arc<Scene>,
        page_index: usize,
        page_unscaled_size: egui::Vec2, // (width, height) in PDF User Space
        page_screen_rect: egui::Rect,  // page bounding box on screen
        viewport_rect: egui::Rect,     // current visible viewport on screen
        zoom: f32,
    ) -> Vec<(egui::TextureId, egui::Rect)> {
        let mut draw_calls = Vec::new();
        let current_ptr = Arc::as_ptr(scene);

        // Limit for whole-page rendering to avoid allocating huge textures.
        // If zoom is high, we transition to adaptive tiling.
        if zoom <= 2.0 {
            let width = page_screen_rect.width().round() as u32;
            let height = page_screen_rect.height().round() as u32;
            let width = width.clamp(1, 4096);
            let height = height.clamp(1, 4096);

            let needs_recreate = if let Some(tex) = self.whole_page_textures.get(&page_index) {
                tex.width != width || tex.height != height
            } else {
                true
            };

            if needs_recreate {
                self.recreate_whole_page_texture(render_state, page_index, width, height);
            }

            let tex = self.whole_page_textures.get_mut(&page_index).unwrap(); // RR-15 Safe: Guaranteed to exist as it is created/recreated immediately above
            tex.last_frame_used = self.current_frame;

            let needs_render = tex.scene_ptr != Some(current_ptr);
            if needs_render {
                let device = &render_state.device;
                let queue = &render_state.queue;

                // Scale transform mapping PDF user space to texture space.
                let scale_x = width as f64 / page_unscaled_size.x as f64;
                let scale_y = height as f64 / page_unscaled_size.y as f64;
                let transform = kurbo::Affine::new([scale_x, 0.0, 0.0, -scale_y, 0.0, height as f64]);

                let mut transformed_scene = Scene::new();
                transformed_scene.append(scene, Some(transform));

                let _ = self.renderer.render_to_texture(
                    device,
                    queue,
                    &transformed_scene,
                    &tex.view,
                    &RenderParams {
                        base_color: vello::peniko::color::palette::css::WHITE,
                        width,
                        height,
                        antialiasing_method: AaConfig::Msaa16,
                    },
                );
                tex.scene_ptr = Some(current_ptr);
            }

            draw_calls.push((tex.egui_texture, page_screen_rect));
        } else {
            // Adaptive Tiling (512x512 grid tiles)
            let tile_size = 512.0f32;
            let cols = (page_screen_rect.width() / tile_size).ceil() as usize;
            let rows = (page_screen_rect.height() / tile_size).ceil() as usize;

            for col in 0..cols {
                for row in 0..rows {
                    let tile_x_min = page_screen_rect.min.x + col as f32 * tile_size;
                    let tile_y_min = page_screen_rect.min.y + row as f32 * tile_size;
                    let tile_w = tile_size.min(page_screen_rect.max.x - tile_x_min);
                    let tile_h = tile_size.min(page_screen_rect.max.y - tile_y_min);

                    if tile_w <= 0.1 || tile_h <= 0.1 {
                        continue;
                    }

                    let tile_rect = egui::Rect::from_min_size(
                        egui::pos2(tile_x_min, tile_y_min),
                        egui::vec2(tile_w, tile_h),
                    );

                    // Viewport Culling: only render tile if it intersects the screen viewport
                    if tile_rect.intersects(viewport_rect) {
                        let tile_tex_w = (tile_w * 2.0).round() as u32;
                        let tile_tex_h = (tile_h * 2.0).round() as u32;
                        let tile_tex_w = tile_tex_w.max(1);
                        let tile_tex_h = tile_tex_h.max(1);

                        let tile_key = (page_index, col, row);

                        let needs_recreate = if let Some(tex) = self.tile_textures.get(&tile_key) {
                            tex.width != tile_tex_w || tex.height != tile_tex_h
                        } else {
                            true
                        };

                        if needs_recreate {
                            self.recreate_tile_texture(render_state, tile_key, tile_tex_w, tile_tex_h);
                        }

                        let tex = self.tile_textures.get_mut(&tile_key).unwrap(); // RR-15 Safe: Guaranteed to exist as it is created/recreated immediately above
                        tex.last_frame_used = self.current_frame;

                        let needs_render = tex.scene_ptr != Some(current_ptr);
                        if needs_render {
                            let device = &render_state.device;
                            let queue = &render_state.queue;

                            // Direct translated tile transform.
                            // Maps PDF coordinates directly to this tile's texture bounds.
                            let sx = zoom * 2.0;
                            let sy = -zoom * 2.0;
                            let tx = (page_screen_rect.min.x - tile_x_min) * 2.0;
                            let ty = (page_screen_rect.min.y - tile_y_min + page_unscaled_size.y * zoom) * 2.0;
                            let transform = kurbo::Affine::new([sx as f64, 0.0, 0.0, sy as f64, tx as f64, ty as f64]);

                            // Custom transform mapping utilizing scene offsets.
                            let mut transformed_scene = Scene::new();
                            transformed_scene.append(scene, Some(transform));

                            let _ = self.renderer.render_to_texture(
                                device,
                                queue,
                                &transformed_scene,
                                &tex.view,
                                &RenderParams {
                                    base_color: vello::peniko::color::palette::css::WHITE,
                                    width: tile_tex_w,
                                    height: tile_tex_h,
                                    antialiasing_method: AaConfig::Msaa16,
                                },
                            );
                            tex.scene_ptr = Some(current_ptr);
                        }

                        draw_calls.push((tex.egui_texture, tile_rect));
                    }
                }
            }
        }

        draw_calls
    }

    fn recreate_whole_page_texture(
        &mut self,
        render_state: &RenderState,
        page_index: usize,
        width: u32,
        height: u32,
    ) {
        let device = &render_state.device;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Vello Target Whole Page Texture {}", page_index)),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        if let Some(old_tex) = self.whole_page_textures.get(&page_index) {
            render_state.renderer.write().free_texture(&old_tex.egui_texture);
        }

        let tid = render_state.renderer.write().register_native_texture(
            device,
            &view,
            wgpu::FilterMode::Linear,
        );

        self.whole_page_textures.insert(
            page_index,
            CachedTexture {
                _texture: texture,
                view,
                egui_texture: tid,
                width,
                height,
                scene_ptr: None,
                last_frame_used: self.current_frame,
            },
        );
    }

    fn recreate_tile_texture(
        &mut self,
        render_state: &RenderState,
        tile_key: (usize, usize, usize),
        width: u32,
        height: u32,
    ) {
        let device = &render_state.device;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Vello Target Tile Texture {:?}", tile_key)),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        if let Some(old_tex) = self.tile_textures.get(&tile_key) {
            render_state.renderer.write().free_texture(&old_tex.egui_texture);
        }

        let tid = render_state.renderer.write().register_native_texture(
            device,
            &view,
            wgpu::FilterMode::Linear,
        );

        self.tile_textures.insert(
            tile_key,
            CachedTexture {
                _texture: texture,
                view,
                egui_texture: tid,
                width,
                height,
                scene_ptr: None,
                last_frame_used: self.current_frame,
            },
        );
    }

    /// Garbage collects unused textures (sublimation) to free GPU memory.
    fn gc_textures(&mut self, render_state: &RenderState) {
        let current = self.current_frame;

        // Evict whole page textures unused for > 60 frames
        let mut to_remove_pages = Vec::new();
        for (&page_index, tex) in &self.whole_page_textures {
            if current - tex.last_frame_used > 60 {
                to_remove_pages.push(page_index);
            }
        }
        for page_index in to_remove_pages {
            if let Some(tex) = self.whole_page_textures.remove(&page_index) {
                render_state.renderer.write().free_texture(&tex.egui_texture);
            }
        }

        // Evict tile textures unused for > 30 frames
        let mut to_remove_tiles = Vec::new();
        for (&key, tex) in &self.tile_textures {
            if current - tex.last_frame_used > 30 {
                to_remove_tiles.push(key);
            }
        }
        for key in to_remove_tiles {
            if let Some(tex) = self.tile_textures.remove(&key) {
                render_state.renderer.write().free_texture(&tex.egui_texture);
            }
        }
    }
}
