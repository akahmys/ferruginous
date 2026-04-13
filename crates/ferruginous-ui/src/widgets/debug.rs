use eframe::egui;
use egui::{Color32, RichText, Frame, Margin};
use crate::FerruginousApp;

/// Renders a floating debug overlay in the bottom-right corner.
pub fn show_debug_overlay(app: &mut FerruginousApp, ctx: &egui::Context) {
    if !app.show_debug_overlay {
        return;
    }

    egui::Window::new("System Diagnostics")
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
        .collapsible(true)
        .resizable(false)
        .frame(Frame::window(&ctx.style())
            .fill(Color32::from_rgba_premultiplied(20, 20, 25, 230))
            .inner_margin(Margin::same(12))
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("HARDWARE").strong().color(Color32::LIGHT_BLUE));
                ui.label(format!("GPU: {}", app.gpu_name));
                
                ui.add_space(8.0);
                ui.label(RichText::new("RENDERING").strong().color(Color32::LIGHT_BLUE));
                let status_color = if app.vello_init_error.is_none() { Color32::GREEN } else { Color32::RED };
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(RichText::new(if app.vello_init_error.is_none() { "READY" } else { "ERROR" }).color(status_color));
                });
                
                ui.label(format!("Frames: {}", app.frame_count));
                ui.label(format!("Callbacks: {}", app.vello_callback_count));
                
                ui.add_space(8.0);
                ui.label(RichText::new("DIAGNOSTICS").strong().color(Color32::LIGHT_BLUE));
                ui.checkbox(&mut app.diagnostic_mode, "Diagnostic Mode");
                
                if ui.button("Hide Overlay").clicked() {
                    app.show_debug_overlay = false;
                }
            });
        });
}
