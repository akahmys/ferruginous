use egui_wgpu::RenderState;
use std::collections::BTreeMap;
use vello::wgpu;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};

struct PageTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    egui_texture: egui::TextureId,
    width: u32,
    height: u32,
}

pub struct VelloRenderer {
    renderer: Renderer,
    page_textures: BTreeMap<usize, PageTexture>,
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

        Some(Self { renderer, page_textures: BTreeMap::new() })
    }

    pub fn render_page(
        &mut self,
        render_state: &RenderState,
        scene: &Scene,
        page_index: usize,
        width: u32,
        height: u32,
    ) -> Option<egui::TextureId> {
        let device = &render_state.device;
        let queue = &render_state.queue;

        let needs_recreate = if let Some(tex) = self.page_textures.get(&page_index) {
            tex.width != width || tex.height != height
        } else {
            true
        };

        if needs_recreate {
            self.recreate_texture(render_state, page_index, width, height);
        }

        let tex = self.page_textures.get(&page_index)?;

        self.renderer
            .render_to_texture(
                device,
                queue,
                scene,
                &tex.view,
                &RenderParams {
                    base_color: vello::peniko::color::palette::css::WHITE,
                    width,
                    height,
                    antialiasing_method: AaConfig::Msaa16,
                },
            )
            .ok()?;

        Some(tex.egui_texture)
    }

    fn recreate_texture(
        &mut self,
        render_state: &RenderState,
        page_index: usize,
        width: u32,
        height: u32,
    ) {
        let device = &render_state.device;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Vello Target Texture {}", page_index)),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        if let Some(old_tex) = self.page_textures.get(&page_index) {
            render_state.renderer.write().free_texture(&old_tex.egui_texture);
        }

        let tid = render_state.renderer.write().register_native_texture(
            device,
            &view,
            wgpu::FilterMode::Linear,
        );

        self.page_textures.insert(
            page_index,
            PageTexture { _texture: texture, view, egui_texture: tid, width, height },
        );
    }
}
