use std::path::Path;
use std::num::NonZeroUsize;
use vello::{Scene, Renderer, RendererOptions, AaSupport, RenderParams, AaConfig};
use vello::util::{RenderContext, block_on_wgpu};
use vello::wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, 
    TexelCopyBufferInfo, TexelCopyBufferLayout, TextureDescriptor, TextureFormat, TextureUsages,
};
use image::{RgbaImage, ImageFormat};

pub async fn render_to_png(scene: &Scene, width: u32, height: u32, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = RenderContext::new();
    let device_id = context
        .device(None)
        .await
        .ok_or("No compatible device found")?;
    
    let device_handle = &mut context.devices[device_id];
    let device = &device_handle.device;
    let queue = &device_handle.queue;

    let mut renderer = Renderer::new(
        device,
        RendererOptions {
            use_cpu: false,
            num_init_threads: NonZeroUsize::new(1),
            antialiasing_support: AaSupport::area_only(),
            ..Default::default()
        },
    ).map_err(|e| format!("Failed to create renderer: {}", e))?;

    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let target = device.create_texture(&TextureDescriptor {
        label: Some("Target texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: vello::wgpu::TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = target.create_view(&vello::wgpu::TextureViewDescriptor::default());

    let render_params = RenderParams {
        base_color: vello::peniko::color::palette::css::WHITE,
        width,
        height,
        antialiasing_method: AaConfig::Area,
    };

    renderer
        .render_to_texture(device, queue, scene, &view, &render_params)
        .map_err(|e| format!("Rendering failed: {}", e))?;

    let padded_byte_width = (width * 4).next_multiple_of(256);
    let buffer_size = padded_byte_width as u64 * height as u64;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Copy buffer"),
        size: buffer_size,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Copy out encoder"),
    });

    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_byte_width),
                rows_per_image: None,
            },
        },
        size,
    );

    queue.submit([encoder.finish()]);

    let buf_slice = buffer.slice(..);
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), vello::wgpu::BufferAsyncError>>();
    buf_slice.map_async(vello::wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });

    let recv_result = block_on_wgpu(device, rx)
        .map_err(|e| format!("Channel closed: {}", e))?;
    recv_result.map_err(|e| format!("Buffer mapping failed: {}", e))?;

    let data = buf_slice.get_mapped_range();
    let mut result_unpadded = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * padded_byte_width) as usize;
        result_unpadded.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }

    let image = RgbaImage::from_raw(width, height, result_unpadded)
        .ok_or("Failed to create RgbaImage")?;
    
    image.save_with_format(path, ImageFormat::Png)?;

    Ok(())
}
