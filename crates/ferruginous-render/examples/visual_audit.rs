#![allow(unused_imports)]
#![allow(missing_docs)]
use bytes::Bytes;
use ferruginous_render::{RenderBackend, headless};
use ferruginous_sdk::PdfDocument;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "samples/bokutokitan.v20.pdf";
    let output_dir = "artifacts";
    std::fs::create_dir_all(output_dir)?;
    let output_path = "artifacts/debug_render_output.png";

    println!("Loading PDF: {}", pdf_path);
    let data = Bytes::from(std::fs::read(pdf_path)?);
    let doc = PdfDocument::open(data)?;

    let page_index = 0;

    let mut backend = ferruginous_render::VelloBackend::new();
    let (p_w, p_h) = doc.get_page_size(page_index).unwrap_or((595.0, 842.0));

    // High-resolution render (200 DPI approx)
    let width = 1654;
    let height = 2339;
    let scale_x = width as f64 / p_w;
    let scale_y = height as f64 / p_h;
    let initial_transform = kurbo::Affine::new([scale_x, 0.0, 0.0, -scale_y, 0.0, p_h * scale_y]);

    doc.render_page(page_index, &mut backend, initial_transform)?;
    let scene = backend.scene();

    // High-resolution render (200 DPI approx)
    let width = 1654;
    let height = 2339;

    headless::render_to_image(
        scene,
        width,
        height,
        Path::new(output_path),
        image::ImageFormat::Png,
    )
    .await?;

    println!("Render complete. Please check {}", output_path);
    Ok(())
}
