//! A simple command-line utility for testing PDF rendering to PNG.
//!
//! This utility takes a PDF path, a 1-indexed page number, and an output path,
//! and renders the specified page using the Vello backend.

use ferruginous_render::VelloBackend;
use ferruginous_sdk::PdfDocument;
use kurbo::Affine;
use std::fs;

#[tokio::main]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
async fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: render_test <pdf_path> <page_number> <output_path>");
        return;
    }

    let pdf_path = &args[1];
    let page_num: usize = args[2].parse().expect("Invalid page number");
    let output_path = &args[3];

    let data = fs::read(pdf_path).expect("Failed to read sample PDF");
    let doc = PdfDocument::open(bytes::Bytes::from(data)).unwrap();
    let (width, height) = doc.get_page_size(page_num - 1).unwrap();
    println!("Page size: {width}x{height}");

    let mut backend = VelloBackend::new(VelloBackend::load_system_fonts());
    let transform = Affine::scale_non_uniform(1.0, -1.0) * Affine::translate((0.0, -height));
    doc.render_page(page_num - 1, &mut backend, transform).unwrap();

    ferruginous_render::headless::render_to_image(
        backend.scene(),
        width as u32,
        height as u32,
        std::path::Path::new(output_path),
        image::ImageFormat::Png,
    )
    .await
    .expect("Failed to render to image");

    println!("Rendered to {output_path}");
}
