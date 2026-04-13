use eframe::egui;
use egui::{Color32, Rect};
use crate::FerruginousApp;
use crate::SUPER_SAMPLE_FACTOR;

/// Renders the primary PDF canvas and paper surface.
pub fn show_canvas(app: &mut FerruginousApp, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(Color32::from_rgb(240, 240, 245)))
        .show(ctx, |ui| {
            render_page_surface(app, ui);
        });
}

fn render_page_surface(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    
    // Calculate paper size with zoom
    let zoom_factor = app.zoom_factor;
    let paper_size = app.current_page_size * zoom_factor;
    
    // Center the paper in the available space, accounting for pan
    let paper_center = rect.center() + app.pan_offset;
    let paper_rect = Rect::from_center_size(paper_center, paper_size);

    // Draw shadow/border for the paper
    ui.painter().rect_filled(
        paper_rect.expand(2.0),
        0.0,
        Color32::from_black_alpha(20)
    );

    // Draw actual white paper surface
    ui.painter().rect_filled(paper_rect, 0.0, Color32::WHITE);

    // Render resulting Vello texture image if available
    if let Some(texture_id) = app.vello_texture_id {
        let u_max = (paper_rect.width() * SUPER_SAMPLE_FACTOR / 2048.0).min(1.0);
        let v_max = (paper_rect.height() * SUPER_SAMPLE_FACTOR / 2048.0).min(1.0);

        ui.painter().image(
            texture_id,
            paper_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(u_max, v_max)),
            Color32::WHITE
        );
    }

    // Handle interactions (Zoom, Pan)
    handle_canvas_interactions(app, ui, rect);
}

fn handle_canvas_interactions(app: &mut FerruginousApp, ui: &mut egui::Ui, rect: Rect) {
    let response = ui.interact(rect, ui.id(), egui::Sense::drag());

    if response.dragged() {
        app.pan_offset += response.drag_delta();
    }

    // Zoom via scroll
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0 {
        let old_zoom = app.zoom_factor;
        app.zoom_factor = (app.zoom_factor + scroll_delta * 0.001).clamp(0.1, 10.0);
        
        // Adjust pan to zoom towards mouse or center
        if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
            let rel_pos = mouse_pos - rect.center() - app.pan_offset;
            let zoom_ratio = app.zoom_factor / old_zoom;
            app.pan_offset -= rel_pos * (zoom_ratio - 1.0);
        }
        
        app.update_rendering();
    }
}
