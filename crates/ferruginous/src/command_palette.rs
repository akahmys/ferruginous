// RR-15 Limit: GUI - Command Palette window declaration and dispatch
pub struct CommandPalette;

impl CommandPalette {
    pub fn show(app: &mut crate::app::FerruginousApp, ctx: &egui::Context) {
        let mut show_palette = app.show_command_palette;
        if !show_palette {
            return;
        }

        let mut close_palette = false;
        egui::Window::new("⌨️ Command Palette")
            .open(&mut show_palette)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 100.0))
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    let text_resp = ui.text_edit_singleline(&mut app.command_palette_search);
                    text_resp.request_focus();
                });
                ui.separator();

                let query = app.command_palette_search.to_lowercase();
                let commands = vec![
                    ("Load PDF", "Load a new PDF document"),
                    ("Reset View", "Reset zoom and pan"),
                    ("Redact Brush", "Toggle Redact Brush tool"),
                    ("Tagging Brush", "Toggle Tagging Brush tool"),
                    ("Caliper Brush", "Toggle Caliper Brush tool"),
                    ("Inspector", "Toggle Arlington PDF Inspector"),
                    ("Export PDF", "Open production export wizard"),
                    ("Reading Order", "Toggle reading order bar overlay"),
                ];

                for (cmd_name, cmd_desc) in commands {
                    if query.is_empty()
                        || cmd_name.to_lowercase().contains(&query)
                        || cmd_desc.to_lowercase().contains(&query)
                    {
                        if ui.selectable_label(false, format!("{} — {}", cmd_name, cmd_desc)).clicked() {
                            match cmd_name {
                                "Load PDF" => {
                                    if let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                                        app.open_file(p, ctx);
                                    }
                                }
                                "Reset View" => app.reset_view(),
                                "Redact Brush" => {
                                    app.redaction_manager.is_active = !app.redaction_manager.is_active;
                                    if app.redaction_manager.is_active {
                                        app.selection_manager.clear();
                                        app.selection_manager.is_tagging_brush_active = false;
                                        app.caliper_tool.is_active = false;
                                    }
                                }
                                "Tagging Brush" => {
                                    app.selection_manager.is_tagging_brush_active = !app.selection_manager.is_tagging_brush_active;
                                    if app.selection_manager.is_tagging_brush_active {
                                        app.selection_manager.clear();
                                        app.redaction_manager.is_active = false;
                                        app.caliper_tool.is_active = false;
                                    }
                                }
                                "Caliper Brush" => {
                                    app.caliper_tool.is_active = !app.caliper_tool.is_active;
                                    if app.caliper_tool.is_active {
                                        app.selection_manager.clear();
                                        app.redaction_manager.is_active = false;
                                        app.selection_manager.is_tagging_brush_active = false;
                                    }
                                }
                                "Inspector" => app.show_inspector = !app.show_inspector,
                                "Export PDF" => app.show_export_wizard = true,
                                "Reading Order" => app.show_reading_order = !app.show_reading_order,
                                _ => {}
                            }
                            close_palette = true;
                        }
                    }
                }
            });

        if close_palette {
            app.show_command_palette = false;
            app.command_palette_search.clear();
        } else {
            app.show_command_palette = show_palette;
        }
    }
}
