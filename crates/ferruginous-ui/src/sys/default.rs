//! Default (fallback) platform implementations.

use eframe::egui;

/// Returns standard eframe options for non-macOS platforms.
pub fn get_native_options() -> eframe::NativeOptions {
    eframe::NativeOptions::default()
}

/// No-op font setup for platforms where system fonts are not configured yet.
pub fn setup_fonts(_ctx: &egui::Context) {
    // Default egui fonts used
}
