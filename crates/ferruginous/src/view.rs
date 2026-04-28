use egui;
use std::collections::HashMap;

#[derive(Clone)]
pub struct PageLayout {
    pub index: usize,
    pub rect: egui::Rect,
}

pub struct PDFView {
    pub zoom: f32,
    pub pan: egui::Vec2,
    pub visible_pages: Vec<usize>,
}

impl PDFView {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            visible_pages: Vec::new(),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        layouts: &[PageLayout],
        textures: &HashMap<usize, egui::TextureId>,
    ) {
        let (rect, response) = ui.allocate_at_least(ui.available_size(), egui::Sense::drag());
        self.handle_input(ui, &response);
        
        // Workspace background (Light Gray)
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(240, 240, 240));

        let mut new_visible = Vec::new();
        let origin = egui::pos2(rect.center().x, rect.min.y + 20.0) + self.pan;

        for layout in layouts {
            let page_rect = egui::Rect::from_min_size(
                origin + layout.rect.min.to_vec2() * self.zoom,
                layout.rect.size() * self.zoom,
            );

            // Viewport culling
            if rect.intersects(page_rect) {
                new_visible.push(layout.index);
                
                // Page Shadow
                ui.painter().rect_filled(page_rect.translate(egui::vec2(2.0, 2.0)), 0.0, egui::Color32::from_black_alpha(50));
                
                // Page Background (White)
                ui.painter().rect_filled(page_rect, 0.0, egui::Color32::WHITE);
                
                if let Some(tid) = textures.get(&layout.index) {
                    ui.painter().image(*tid, page_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                } else {
                    ui.painter().text(page_rect.center(), egui::Align2::CENTER_CENTER, format!("Loading Page {}...", layout.index + 1), egui::FontId::proportional(20.0), egui::Color32::GRAY);
                }
            }
        }
        
        self.visible_pages = new_visible;
        if response.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
    }

    fn handle_input(&mut self, ui: &mut egui::Ui, response: &egui::Response) {
        ui.input(|i| {
            let zoom_delta = i.zoom_delta();
            if zoom_delta != 1.0 { self.zoom = (self.zoom * zoom_delta).clamp(0.1, 10.0); }
            let scroll_delta = i.smooth_scroll_delta;
            if i.modifiers.command && scroll_delta.y != 0.0 {
                self.zoom = (self.zoom * (scroll_delta.y * 0.005).exp()).clamp(0.1, 10.0);
            }
            if !i.modifiers.command { self.pan += scroll_delta; }
        });
        if response.dragged() { self.pan += response.drag_delta(); }
    }
}
