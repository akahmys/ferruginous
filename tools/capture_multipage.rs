//! Automated multi-page PDF rendering capture tool.
use std::fs;
use ferruginous_sdk::loader;
use ferruginous_render::visual_harness::HeadlessDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_path = "samples/legacy/antenna-house-samples.pdf";
    let output_dir = "artifacts/screenshots/multipage";
    fs::create_dir_all(output_dir)?;

    let dev = HeadlessDevice::new().map_err(|e| format!("Failed to init GPU: {e}"))?;

    println!("Loading: {sample_path}");
    let data = fs::read(sample_path)?;
    let doc = loader::load_document_structure(&data)?;
    let page_tree = doc.page_tree()?;
    let total_pages = page_tree.get_count();
    println!("Total pages: {total_pages}");

    // Sample every 15 pages
    let mut focus_pages = Vec::new();
    for i in (0..total_pages).step_by(15) {
        focus_pages.push(i);
    }
    // Also include the last page if not already there
    if let Some(&last) = focus_pages.last() {
        if last != total_pages - 1 {
            focus_pages.push(total_pages - 1);
        }
    }

    for &page_idx in &focus_pages {
        println!("Processing Page {}...", page_idx + 1);
        let page = match page_tree.get_page(page_idx) {
            Ok(p) => p,
            Err(e) => {
                println!("  Error getting page {page_idx}: {e:?}");
                continue;
            }
        };
        
        let bbox = match page.media_box_array() {
            Some(b) => [b[0], b[1], b[2], b[3]],
            None => [0.0, 0.0, 595.0, 842.0],
        };

        let width = (bbox[2] - bbox[0]).abs() as u32;
        let height = (bbox[3] - bbox[1]).abs() as u32;
        
        let render_width = 1000u32;
        let scale = render_width as f32 / width.max(1) as f32;
        let render_height = (height as f32 * scale) as u32;

        let display_list = page.get_display_list()?;
        
        println!("  Rendering {render_width}x{render_height}...");
        let img = dev.capture_rendering(&display_list, render_width, render_height)?;
        
        let out_path = format!("{}/antenna_house_p{}.png", output_dir, page_idx + 1);
        img.save(&out_path)?;
        println!("  Saved to: {out_path}");
    }

    Ok(())
}
