//! Regression test for verifying vertical writing mode rendering.
use std::fs;
use ferruginous_sdk::loader;
use ferruginous_render::visual_harness::HeadlessDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_data = fs::read("samples/legacy/writing-mode-1.pdf")?;
    let doc = loader::load_document_structure(&pdf_data)?;
    let page_tree = doc.page_tree()?;
    let dev = HeadlessDevice::new().map_err(|e| format!("Failed to init GPU: {e}"))?;
    
    let page = page_tree.get_page(0)?;
    let display_list = page.get_display_list()?;
    
    let image = dev.capture_rendering(&display_list, 1000, 1000)?;
    fs::create_dir_all("artifacts/screenshots")?;
    image.save("artifacts/screenshots/writing_mode_1_final.png")?;
    
    println!("Saved to: artifacts/screenshots/writing_mode_1_final.png");
    Ok(())
}
