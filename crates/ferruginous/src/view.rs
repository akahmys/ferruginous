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



    pub fn center_on_rect(&mut self, viewport_rect: egui::Rect, page_layout: &PageLayout, rect: [f32; 4]) {
        let pdf_center_x = (rect[0] + rect[2]) / 2.0;
        let pdf_center_y = (rect[1] + rect[3]) / 2.0;
        
        let unscaled_h = page_layout.rect.height();
        
        // Convert to egui page-local coordinate system (Y=0 is top)
        let local_x = pdf_center_x;
        let local_y = unscaled_h - pdf_center_y;
        
        // In virtual space (relative to layout center/top):
        let page_local_pos = page_layout.rect.min + egui::vec2(local_x, local_y);
        
        // We want origin + page_local_pos * zoom = viewport_rect.center()
        let origin_no_pan = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0);
        self.pan = viewport_rect.center().to_vec2() - origin_no_pan.to_vec2() - page_local_pos.to_vec2() * self.zoom;
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
        signature_highlight: &Option<(usize, egui::Rect)>,
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
                        
                        // Render visually accurate high-contrast redacted text overlay for preview mode
                        if redact_rect.width() > 60.0 && redact_rect.height() > 12.0 {
                            ui.painter().text(
                                redact_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "[REDACTED]",
                                egui::FontId::monospace(9.0),
                                egui::Color32::from_rgb(255, 75, 75), // Coral high-visibility red
                            );
                        }
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
                        let time = ui.ctx().input(|i| i.time);
                        let pulse = (time * 6.0).sin().abs() as f32; // Pulse between 0.0 and 1.0
                        let outline_color = egui::Color32::from_rgb(255, 165, 0);
                        let fill_opacity = 20 + (pulse * 35.0) as u8;
                        let stroke_w = 2.0 + pulse * 2.0;

                        ui.painter().rect_stroke(
                            *highlight_rect,
                            0.0,
                            egui::Stroke::new(stroke_w, outline_color),
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().rect_filled(
                            *highlight_rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(255, 165, 0, fill_opacity),
                        );
                        
                        // Request continuous repaint for smooth pulsing micro-animation!
                        ui.ctx().request_repaint();
                    }
                }

                // Render visual digital signature placement field beautifully if set
                if let Some((sig_page, sig_rect)) = signature_highlight {
                    if *sig_page == layout.index {
                        // Draw beautiful semi-transparent gold/amber background with double borders
                        ui.painter().rect_filled(
                            *sig_rect,
                            4.0,
                            egui::Color32::from_rgba_unmultiplied(226, 135, 67, 30), // Gold/Amber semi-translucent fill
                        );
                        ui.painter().rect_stroke(
                            *sig_rect,
                            4.0,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(226, 135, 67)), // Solid Gold border
                            egui::StrokeKind::Outside,
                        );

                        // Draw diagonal crossing lines
                        ui.painter().line_segment(
                            [sig_rect.left_top(), sig_rect.right_bottom()],
                            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(226, 135, 67, 100)),
                        );
                        ui.painter().line_segment(
                            [sig_rect.right_top(), sig_rect.left_bottom()],
                            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(226, 135, 67, 100)),
                        );

                        // Render text description
                        ui.painter().text(
                            sig_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "🔏 [ DIGITAL SIGNATURE FIELD ]",
                            egui::FontId::monospace(12.0),
                            egui::Color32::from_rgb(226, 135, 67),
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
