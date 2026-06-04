#![allow(
    clippy::collapsible_if,
    clippy::match_result_ok,
    clippy::too_many_arguments,
    clippy::large_enum_variant,
    clippy::unnecessary_unwrap,
    clippy::needless_borrow,
    clippy::equatable_if_let,
    clippy::uninlined_format_args,
    clippy::ref_option,
    clippy::fn_params_excessive_bools,
    clippy::needless_pass_by_ref_mut,
    clippy::float_cmp,
    clippy::semicolon_if_nothing_returned,
    clippy::map_unwrap_or,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::manual_midpoint,
    clippy::cast_lossless,
    clippy::suboptimal_flops,
    clippy::cast_precision_loss,
    clippy::struct_excessive_bools,
    clippy::cloned_instead_of_copied,
    clippy::unnecessary_debug_formatting,
    clippy::stable_sort_primitive,
    clippy::derive_partial_eq_without_eq,
    clippy::imprecise_flops,
    clippy::needless_lifetimes,
    clippy::result_large_err,
    dead_code,
    missing_docs
)]

mod app;
mod cad_canvas;
mod command_palette;
mod export_wizard;
mod inspector;
mod interaction;
mod locale;
mod redaction;
mod redaction_studio;
mod sidebar;
mod thumbnail_sidebar;
mod vello_egui;
mod view;
mod worker;

use app::FerruginousApp;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let pdf_path = std::env::args().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
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
