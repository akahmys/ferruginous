use eframe::egui;
use egui::{Color32, RichText, Frame, Stroke, Margin, Button, Vec2};
use crate::FerruginousApp;
use crate::types::ToolMode;

/// Renders the minimalist top header panel.
pub fn show_header(app: &mut FerruginousApp, ctx: &egui::Context) {
    let border_color = Color32::from_rgb(235, 235, 240);
    let bg_fill = Color32::WHITE;

    egui::TopBottomPanel::top("header_premium")
        .frame(Frame::default()
            .fill(bg_fill)
            .stroke(Stroke::new(1.0, border_color))
            .inner_margin(Margin::symmetric(20, 12)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                show_brand(ui);
                ui.add_space(32.0);
                
                show_tool_modes(app, ui);
                ui.add_space(24.0);
                ui.separator();
                ui.add_space(24.0);

                show_navigation(app, ui);
                ui.add_space(24.0);
                ui.separator();
                ui.add_space(24.0);

                show_zoom(app, ui);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    show_utility_actions(app, ui);
                });
            });
        });
}

fn show_brand(ui: &mut egui::Ui) {
    let rust = Color32::from_rgb(183, 65, 14);
    ui.label(RichText::new("FERRUGINOUS").color(rust).strong().size(15.0).extra_letter_spacing(2.0));
}

fn show_tool_modes(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    let rust = Color32::from_rgb(183, 65, 14);
    ui.horizontal(|ui| {
        for &mode in &[ToolMode::Select, ToolMode::Snap, ToolMode::Measure] {
            let is_selected = app.tool_mode == mode;
            let text = RichText::new(mode.label().to_uppercase());
            let resp = ui.selectable_label(is_selected, text);
            if resp.clicked() {
                app.tool_mode = mode;
            }
            if is_selected {
                let rect = resp.rect;
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(2.0, rust)
                );
            }
        }
    });
}

fn show_navigation(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.add(Button::new("前へ").frame(false)).clicked() && app.current_page > 0 {
            app.current_page -= 1;
            app.pan_offset = Vec2::ZERO;
            app.update_rendering();
        }
        
        ui.add_space(8.0);
        ui.label(RichText::new(format!("{} / {}", app.current_page + 1, app.page_count)).weak());
        ui.add_space(8.0);

        if ui.add(Button::new("次へ").frame(false)).clicked() && app.current_page + 1 < app.page_count {
            app.current_page += 1;
            app.pan_offset = Vec2::ZERO;
            app.update_rendering();
        }
    });
}

fn show_zoom(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.add(Button::new("縮小").frame(false)).clicked() {
            app.zoom_factor = (app.zoom_factor - 0.1).clamp(0.1, 10.0);
            app.update_rendering();
        }
        
        ui.add_space(4.0);
        ui.label(RichText::new(format!("{:.0}%", app.zoom_factor * 100.0)).strong());
        ui.add_space(4.0);

        if ui.add(Button::new("拡大").frame(false)).clicked() {
            app.zoom_factor = (app.zoom_factor + 0.1).clamp(0.1, 10.0);
            app.update_rendering();
        }

        ui.add_space(12.0);
        
        if ui.add(Button::new("幅に合わせる").frame(false)).clicked() {
            // Target width slightly less than available
            let available_width = ui.available_width().max(800.0);
            app.zoom_factor = (available_width / 850.0).clamp(0.5, 3.0);
            app.pan_offset = Vec2::ZERO;
            app.update_rendering();
        }

        if ui.add(Button::new("リセット").frame(false)).clicked() {
            app.zoom_factor = 1.0;
            app.pan_offset = Vec2::ZERO;
            app.update_rendering();
        }
    });
}

fn show_utility_actions(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    if ui.button(RichText::new("PDFを開く").color(Color32::WHITE)).clicked() {
        app.open_pdf();
    }
    
    ui.add_space(12.0);
    
    if let Some(err) = &app.error_message {
        ui.label(RichText::new(err).color(Color32::RED).size(11.0));
    }
}
