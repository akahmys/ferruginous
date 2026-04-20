mod app;
mod view;

use app::FerruginousApp;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let pdf_path = std::env::args().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Ferruginous"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "ferruginous",
        native_options,
        Box::new(|cc| {
            let mut app = FerruginousApp::new(cc);
            if let Some(path) = pdf_path {
                app.open_file(path, &cc.egui_ctx);
            }
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}
