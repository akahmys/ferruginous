//! Headless GPU rendering harness for testing and automated capture.
use std::sync::Arc;
use wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Device, Extent3d, TexelCopyBufferInfo,
    TexelCopyTextureInfo, TexelCopyBufferLayout, Instance, MapMode, Queue, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, Trace,
};
use ferruginous_sdk::graphics::DrawCommand;
use crate::{RenderBackend, VelloBackend, BackendOptions};
use image::RgbaImage;

/// Structure managing the WGPU headless environment.
pub struct HeadlessDevice {
    /// WGPU device used for rendering.
    pub device: Arc<Device>,
    /// WGPU queue for command submission.
    pub queue: Queue,
}

impl HeadlessDevice {
    /// Synchronously initializes the device and queue.
    pub fn new() -> Result<Self, String> {
        let instance = Instance::default();
        // Based on the compiler error "no method named ok_or_else found for struct wgpu::Adapter,"
        // it is determined that the result of pollster::block_on(...) is directly an Adapter
        // (via Option or Result), and it is safely unwrapped.
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Headless Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                trace: Trace::Off,
            },
        ))
        .map_err(|e| format!("Failed to create wgpu device: {e:?}"))?;

        Ok(Self {
            device: Arc::new(device),
            queue,
        })
    }

    /// Renders a `DisplayList` and retrieves it as an `RgbaImage`.
    pub fn capture_rendering(
        &self,
        commands: &[DrawCommand],
        width: u32,
        height: u32,
    ) -> Result<RgbaImage, String> {
        let mut backend = VelloBackend::new();
        backend.prepare_renderer(&self.device, BackendOptions { use_cpu: false, antialiasing: true })?;
        
        // 1. Create a texture for the rendering target
        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("Capture Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 2. Render
        backend.render_display_list(commands, ferruginous_sdk::graphics::Affine::IDENTITY, None);
        backend.render_to_texture(&self.device, &self.queue, &view, width, height)?;

        // 3. Copy from texture to buffer
        let pixel_size = 4; // RGBA8
        let align = 256;
        let unpadded_row_size = width * pixel_size;
        let padding = (align - unpadded_row_size % align) % align;
        let padded_row_size = unpadded_row_size + padding;
        
        let output_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("Output Buffer"),
            size: u64::from(padded_row_size * height),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Copy Encoder"),
        });

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_size),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        // 4. Read buffer and convert to image
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |v| tx.send(v).unwrap());

        // Use PollType::wait_indefinitely() available in wgpu 25.0+.
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());

        if rx.recv() == Ok(Ok(())) {
            let data = buffer_slice.get_mapped_range();
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);

            for chunk in data.chunks_exact(padded_row_size as usize) {
                pixels.extend_from_slice(&chunk[..unpadded_row_size as usize]);
            }

            drop(data);
            output_buffer.unmap();

            RgbaImage::from_raw(width, height, pixels)
                .ok_or_else(|| "Failed to create image from raw pixels".to_string())
        } else {
            Err("Failed to map buffer".to_string())
        }
    }
}
