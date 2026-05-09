use ferruginous_render::VelloBackend;
use ferruginous_render::headless::render_to_image;
use ferruginous_sdk::Interpreter;
use ferruginous_sdk::PdfDocument;
use image::ImageFormat;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples_dir = Path::new("samples");
    let entries = std::fs::read_dir(samples_dir)?;

    // Ensure the output directory exists
    let output_dir = Path::new("artifacts/renders");
    std::fs::create_dir_all(output_dir)?;

    let system_fonts = VelloBackend::load_system_fonts();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            let filename = path.file_name().unwrap().to_str().unwrap();
            println!("Rendering {}...", filename);

            let data = std::fs::read(&path)?;
            let doc = PdfDocument::open(data.into())?;

            if doc.page_count()? > 0 {
                let page = doc.get_page(0)?;
                let mut backend = VelloBackend::new(system_fonts.clone());

                // Get media box
                let media_box = page.media_box();
                let width = media_box.width() as u32;
                let height = media_box.height() as u32;

                if width == 0 || height == 0 {
                    println!("  Skipping {} due to zero dimension", filename);
                    continue;
                }

                // PDF coordinates are bottom-up, Vello is top-down.
                let transform = kurbo::Affine::scale_non_uniform(1.0, -1.0)
                    * kurbo::Affine::translate((0.0, -(height as f64)));

                let mut interpreter =
                    Interpreter::new(&mut backend, doc.inner(), page.resources_handle(), transform);
                for contents_h in page.contents_handles() {
                    let _ = interpreter.execute(contents_h);
                }

                let output_path = output_dir.join(format!("{}.png", filename));
                render_to_image(backend.scene(), width, height, &output_path, ImageFormat::Png)
                    .await?;
                println!("  Saved to {:?}", output_path);
            }
        }
    }

    Ok(())
}
