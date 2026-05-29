use std::collections::BTreeMap;

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
        Self { zoom: 1.0, pan: egui::Vec2::ZERO, visible_pages: Vec::new() }
    }



    pub fn show_virtual(
        &mut self,
        ui: &mut egui::Ui,
        layouts: &[PageLayout],
        draw_calls: &BTreeMap<usize, Vec<(egui::TextureId, egui::Rect)>>,
        highlights: &BTreeMap<usize, Vec<egui::Rect>>,
        redaction_highlights: &BTreeMap<usize, Vec<egui::Rect>>,
        active_redaction_drag: &Option<(usize, egui::Rect)>,
        structural_highlight: &Option<(usize, egui::Rect)>,
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
                ui.painter().rect_filled(
                    page_rect.translate(egui::vec2(2.0, 2.0)),
                    0.0,
                    egui::Color32::from_black_alpha(50),
                );

                // Page Background (White)
                ui.painter().rect_filled(page_rect, 0.0, egui::Color32::WHITE);

                if let Some(calls) = draw_calls.get(&layout.index) {
                    for &(tid, draw_rect) in calls {
                        ui.painter().image(
                            tid,
                            draw_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    ui.painter().text(
                        page_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Loading Page {}...", layout.index + 1),
                        egui::FontId::proportional(20.0),
                        egui::Color32::GRAY,
                    );
                }

                // Render selection highlights
                if let Some(hl_rects) = highlights.get(&layout.index) {
                    for hl_rect in hl_rects {
                        ui.painter().rect_filled(
                            *hl_rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(0, 120, 215, 60), // Blue translucent highlight
                        );
                    }
                }

                // Render solid black redaction highlights
                if let Some(redact_rects) = redaction_highlights.get(&layout.index) {
                    for redact_rect in redact_rects {
                        ui.painter().rect_filled(
                            *redact_rect,
                            0.0,
                            egui::Color32::BLACK,
                        );
                    }
                }

                // Render active red translucent redaction drag box if it belongs to this page
                if let Some((active_page, drag_rect)) = active_redaction_drag {
                    if *active_page == layout.index {
                        ui.painter().rect_filled(
                            *drag_rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(255, 0, 0, 100), // Red translucent
                        );
                        ui.painter().rect_stroke(
                            *drag_rect,
                            0.0,
                            egui::Stroke::new(1.5, egui::Color32::RED),
                            egui::StrokeKind::Outside,
                        );
                    }
                }

                // Render structural highlight (orange outline with translucent fill) if selected in sidebar
                if let Some((highlight_page, highlight_rect)) = structural_highlight {
                    if *highlight_page == layout.index {
                        ui.painter().rect_stroke(
                            *highlight_rect,
                            0.0,
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 165, 0)), // Orange-Yellow outline
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().rect_filled(
                            *highlight_rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(255, 165, 0, 30), // Translucent orange fill
                        );
                    }
                }
            }
        }

        self.visible_pages = new_visible;
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }

    fn handle_input(&mut self, ui: &mut egui::Ui, response: &egui::Response) {
        ui.input(|i| {
            let zoom_delta = i.zoom_delta();
            if zoom_delta != 1.0 {
                self.zoom = (self.zoom * zoom_delta).clamp(0.1, 10.0);
            }
            let scroll_delta = i.smooth_scroll_delta;
            if i.modifiers.command && scroll_delta.y != 0.0 {
                self.zoom = (self.zoom * (scroll_delta.y * 0.005).exp()).clamp(0.1, 10.0);
            }
            if !i.modifiers.command {
                self.pan += scroll_delta;
            }
        });
        if response.dragged() {
            self.pan += response.drag_delta();
        }
    }
}
