use egui::mutex::Mutex;
use std::sync::Arc;
use vello::Scene;

pub struct PDFView {
    #[allow(dead_code)]
    pub scene: Arc<Mutex<Scene>>,
    pub zoom: f32,
    pub pan: egui::Vec2,
    pub texture: Option<egui::TextureHandle>,
}

impl PDFView {
    pub fn new() -> Self {
        Self {
            scene: Arc::new(Mutex::new(Scene::new())),
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            texture: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_at_least(ui.available_size(), egui::Sense::drag());

        // Handle Input
        ui.input(|i| {
            // Zoom: Command + Mousewheel or Pinch
            let zoom_delta = i.zoom_delta();
            if zoom_delta != 1.0 {
                self.zoom *= zoom_delta;
                self.zoom = self.zoom.clamp(0.1, 10.0);
            }

            // Fallback for Cmd + Scroll if zoom_delta isn't enough on some platforms
            let scroll_delta = i.smooth_scroll_delta;
            if i.modifiers.command && scroll_delta.y != 0.0 {
                let zoom_factor = (scroll_delta.y * 0.005).exp();
                self.zoom *= zoom_factor;
                self.zoom = self.zoom.clamp(0.1, 10.0);
            }

            // Pan: Scroll (without Command) or Drag
            if !i.modifiers.command {
                self.pan += scroll_delta;
            }
        });

        if response.dragged() {
            self.pan += response.drag_delta();
        }

        // App Background
        ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);

        // PDF Page Placement
        let base_page_size = egui::vec2(595.0, 842.0); // A4 Default
        let page_size = base_page_size * self.zoom;
        let page_rect = egui::Rect::from_center_size(rect.center() + self.pan, page_size);

        // White Background & Shadow
        ui.painter().rect_filled(
            page_rect.translate(egui::vec2(2.0, 2.0)),
            0.0,
            egui::Color32::from_black_alpha(100),
        );
        ui.painter().rect_filled(page_rect, 0.0, egui::Color32::WHITE);

        // Render PDF Content Texture (Image Bridge)
        if let Some(texture) = &self.texture {
            ui.painter().image(
                texture.id(),
                page_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            ui.painter().text(
                page_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Rendering Page...",
                egui::FontId::proportional(20.0),
                egui::Color32::GRAY,
            );
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }
}
