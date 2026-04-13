#![allow(clippy::all, missing_docs)]
//! Visual regression tests.
#![allow(clippy::all, missing_docs)]

use ferruginous_render::visual_harness::HeadlessDevice;
use ferruginous_sdk::loader::load_document_structure;
use std::path::Path;
use image::{RgbaImage};

fn run_visual_regression_test(pdf_path: &str, page_idx: usize, name: &str) {
    let baseline_dir = "../../tests/fixtures/baselines";
    let baseline_path = format!("{}/{}-p{}.png", baseline_dir, name, page_idx + 1);
    
    // 1. PDF のロードと指定ページの DisplayList 取得
    let pdf_data = std::fs::read(pdf_path).expect("Failed to read sample PDF");
    let doc = load_document_structure(&pdf_data).expect("Failed to load PDF structure");
    let tree = doc.page_tree().expect("Failed to get page tree");
    let page = tree.get_page(page_idx).expect("Failed to get page");
    let display_list = page.get_display_list().expect("Failed to get display list");
    
    // MediaBox からレンダリングサイズを決定
    let bbox = page.media_box_array().unwrap_or([0.0, 0.0, 595.0, 842.0]);
    let width = (bbox[2] - bbox[0]).abs() as u32;
    let height = (bbox[3] - bbox[1]).abs() as u32;

    // 2. ヘッドレスレンダリングの実行
    let harness = HeadlessDevice::new().expect("Failed to initialize headless WGPU");
    let captured = harness.capture_rendering(&display_list, width, height)
        .expect("Failed to capture rendering");

    // 3. 基線（Baseline）との比較
    let update_baselines = std::env::var("UPDATE_BASELINES").is_ok();
    
    if !Path::new(&baseline_path).exists() || update_baselines {
        std::fs::create_dir_all(baseline_dir).expect("Failed to create baseline dir");
        captured.save(&baseline_path).expect("Failed to save baseline image");
        println!("Baseline updated: {baseline_path}");
        return;
    }

    let baseline = image::open(&baseline_path)
        .expect("Failed to open baseline image")
        .to_rgba8();

    assert_eq!(captured.dimensions(), baseline.dimensions(), "Image dimensions mismatch for {name}");

    let mut diff_count = 0;
    let mut diff_image = RgbaImage::new(width, height);

    for (x, y, p_cap) in captured.enumerate_pixels() {
        let p_base = baseline.get_pixel(x, y);
        if p_cap != p_base {
            diff_count += 1;
            diff_image.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
        } else {
            let mut p = *p_cap;
            p.0[3] = 50; 
            diff_image.put_pixel(x, y, p);
        }
    }

    if diff_count > 0 {
        let diff_out = format!("tests/artifacts/{}-p{}-diff.png", name, page_idx + 1);
        std::fs::create_dir_all("tests/artifacts").unwrap();
        diff_image.save(&diff_out).unwrap();
        panic!(
            "Visual regression failed for {} page {}! {} pixels differ. Diff saved to {}",
            name, page_idx + 1, diff_count, diff_out
        );
    }
}

#[test]
fn test_visual_regression_japanese_text() {
    run_visual_regression_test("../../samples/pdf20/jp-harness.pdf", 0, "jp-harness");
}

#[test]
fn test_visual_regression_graphics_suite() {
    let pdf_path = "../../samples/graphics/graphics-suite.pdf";
    // Page 1: Basic Shapes
    run_visual_regression_test(pdf_path, 0, "graphics-suite");
    // Page 2: Curves & Clipping
    run_visual_regression_test(pdf_path, 1, "graphics-suite");
    // Page 3: Line Styles
    run_visual_regression_test(pdf_path, 2, "graphics-suite");
    // Page 4: Colors
    run_visual_regression_test(pdf_path, 3, "graphics-suite");
    // Page 5: Transparency
    run_visual_regression_test(pdf_path, 4, "graphics-suite");
    // Page 6: Shading
    run_visual_regression_test(pdf_path, 5, "graphics-suite");
    run_visual_regression_test(pdf_path, 6, "graphics-suite");
    run_visual_regression_test(pdf_path, 7, "graphics-suite");
}
