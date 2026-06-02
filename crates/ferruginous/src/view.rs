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

    pub fn show_virtual( // RR-15 Limit: GUI - Renders a virtualized grid layout of PDF pages and overlays highlights/signals
        &mut self,
        ui: &mut egui::Ui,
        layouts: &[PageLayout],
        viewport_texture_id: Option<egui::TextureId>,
        viewport_rect: egui::Rect, // Unified viewport rect from app.rs
        scenes: &std::collections::BTreeMap<usize, std::sync::Arc<vello::Scene>>,
        highlights: &BTreeMap<usize, Vec<egui::Rect>>,
        redaction_highlights: &BTreeMap<usize, Vec<egui::Rect>>,
        active_redaction_drag: &Option<(usize, egui::Rect)>,
        structural_highlight: &Option<(usize, egui::Rect)>,
        signature_highlight: &Option<(usize, egui::Rect)>,
    ) {
        let response = ui.allocate_rect(viewport_rect, egui::Sense::drag());
        self.handle_input(ui, &response);

        // Completely disable egui's default focus ring/outline to prevent flashing orange/red borders during interactions or page switches
        ui.visuals_mut().selection.stroke = egui::Stroke::NONE;

        // 1. Workspace background (Premium Light Gray Theme matching sidebars)
        let bg_color = egui::Color32::from_rgb(235, 237, 240); // Clean, elegant light-slate gray matching the light theme
        ui.painter().rect_filled(viewport_rect, 0.0, bg_color);

        // Draw premium design/CAD grid lines that dynamically move with the pan offset
        let grid_size = 32.0;
        let grid_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10));
        
        // Vertical grid lines
        let mut x = viewport_rect.min.x + (self.pan.x % (grid_size * self.zoom));
        while x < viewport_rect.max.x {
            ui.painter().line_segment(
                [egui::pos2(x, viewport_rect.min.y), egui::pos2(x, viewport_rect.max.y)],
                grid_stroke,
            );
            x += grid_size * self.zoom;
        }
        
        // Horizontal grid lines
        let mut y = viewport_rect.min.y + (self.pan.y % (grid_size * self.zoom));
        while y < viewport_rect.max.y {
            ui.painter().line_segment(
                [egui::pos2(viewport_rect.min.x, y), egui::pos2(viewport_rect.max.x, y)],
                grid_stroke,
            );
            y += grid_size * self.zoom;
        }

        // 2. Draw page shadows and authoritatively paint solid pure-white backings under each visible page
        let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.pan;
        for layout in layouts {
            let page_rect = egui::Rect::from_min_size(
                origin + layout.rect.min.to_vec2() * self.zoom,
                layout.rect.size() * self.zoom,
            );
            if viewport_rect.intersects(page_rect) {
                // Draw a beautiful soft blurred/drop shadow for premium depth (drawn *behind* the page backing)
                if scenes.contains_key(&layout.index) {
                    for offset in 1..=4 {
                        ui.painter().rect_filled(
                            page_rect.translate(egui::vec2(offset as f32 * 1.5, offset as f32 * 1.5)),
                            4.0,
                            egui::Color32::from_black_alpha(20 - offset * 4),
                        );
                    }
                }

                // Pure white page backing
                ui.painter().rect_filled(page_rect, 0.0, egui::Color32::WHITE);
            }
        }

        // 3. Draw the single unified viewport texture covering the document panel workspace
        if let Some(tid) = viewport_texture_id {
            ui.painter().image(
                tid,
                viewport_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        let mut new_visible = Vec::new();
        let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.pan;

        for layout in layouts {
            let page_rect = egui::Rect::from_min_size(
                origin + layout.rect.min.to_vec2() * self.zoom,
                layout.rect.size() * self.zoom,
            );

            // Viewport culling
            if viewport_rect.intersects(page_rect) {
                new_visible.push(layout.index);

                if !scenes.contains_key(&layout.index) {
                    // Soft premium white backing for the rendering page to completely remove the gray mask
                    ui.painter().rect_filled(
                        page_rect,
                        4.0,
                        egui::Color32::WHITE,
                    );

                    // Faint, clean border for pristine CAD-like presentation
                    ui.painter().rect_stroke(
                        page_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 224, 230)),
                        egui::StrokeKind::Inside,
                    );

                    // Premium, elegant rendering status indicator
                    ui.painter().text(
                        page_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("⌛ Rendering Page {}...", layout.index + 1),
                        egui::FontId::proportional(15.0),
                        egui::Color32::from_rgb(100, 110, 125),
                    );
                }

                // Render overlays
                self.draw_selection_highlights(ui, layout.index, highlights);
                self.draw_redaction_highlights(ui, layout.index, redaction_highlights);
                self.draw_active_redaction_drag(ui, layout.index, active_redaction_drag);
                self.draw_structural_highlight(ui, layout.index, structural_highlight);
                self.draw_signature_highlight(ui, layout.index, signature_highlight);
            }
        }

        self.visible_pages = new_visible;
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }

    fn draw_selection_highlights(
        &self,
        ui: &mut egui::Ui,
        page_index: usize,
        highlights: &BTreeMap<usize, Vec<egui::Rect>>,
    ) {
        if let Some(hl_rects) = highlights.get(&page_index) {
            for hl_rect in hl_rects {
                ui.painter().rect_filled(
                    *hl_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 120, 215, 60),
                );
            }
        }
    }

    fn draw_redaction_highlights(
        &self,
        ui: &mut egui::Ui,
        page_index: usize,
        redaction_highlights: &BTreeMap<usize, Vec<egui::Rect>>,
    ) {
        if let Some(redact_rects) = redaction_highlights.get(&page_index) {
            for redact_rect in redact_rects {
                ui.painter().rect_filled(*redact_rect, 0.0, egui::Color32::BLACK);
                if redact_rect.width() > 60.0 && redact_rect.height() > 12.0 {
                    ui.painter().text(
                        redact_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "[REDACTED]",
                        egui::FontId::monospace(9.0),
                        egui::Color32::from_rgb(255, 75, 75),
                    );
                }
            }
        }
    }

    fn draw_active_redaction_drag(
        &self,
        ui: &mut egui::Ui,
        page_index: usize,
        active_redaction_drag: &Option<(usize, egui::Rect)>,
    ) {
        if let Some((active_page, drag_rect)) = active_redaction_drag {
            if *active_page == page_index {
                ui.painter().rect_filled(
                    *drag_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(255, 0, 0, 100),
                );
                ui.painter().rect_stroke(
                    *drag_rect,
                    0.0,
                    egui::Stroke::new(1.5, egui::Color32::RED),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }

    fn draw_structural_highlight(
        &self,
        ui: &mut egui::Ui,
        page_index: usize,
        structural_highlight: &Option<(usize, egui::Rect)>,
    ) {
        if let Some((highlight_page, highlight_rect)) = structural_highlight {
            if *highlight_page == page_index {
                let time = ui.ctx().input(|i| i.time);
                let pulse = (time * 6.0).sin().abs() as f32;
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
                ui.ctx().request_repaint();
            }
        }
    }

    fn draw_signature_highlight(
        &self,
        ui: &mut egui::Ui,
        page_index: usize,
        signature_highlight: &Option<(usize, egui::Rect)>,
    ) {
        if let Some((sig_page, sig_rect)) = signature_highlight {
            if *sig_page == page_index {
                ui.painter().rect_filled(
                    *sig_rect,
                    4.0,
                    egui::Color32::from_rgba_unmultiplied(226, 135, 67, 30),
                );
                ui.painter().rect_stroke(
                    *sig_rect,
                    4.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(226, 135, 67)),
                    egui::StrokeKind::Outside,
                );
                ui.painter().line_segment(
                    [sig_rect.left_top(), sig_rect.right_bottom()],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(226, 135, 67, 100)),
                );
                ui.painter().line_segment(
                    [sig_rect.right_top(), sig_rect.left_bottom()],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(226, 135, 67, 100)),
                );
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
