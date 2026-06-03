use crate::worker::WorkerRequest;
use crate::sidebar::USTRegistry;

// RR-15 Limit: GUI - Export Wizard window declaration and layout tree
pub struct ExportWizard;

impl ExportWizard {
    pub fn show(app: &mut crate::app::FerruginousApp, ctx: &egui::Context) {
        let mut open = app.show_export_wizard;
        if !open {
            return;
        }

        let mut should_close = false;
        egui::Window::new("💾 Production Studio Export Wizard")
            .open(&mut open)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                Self::render_compliance_checkboxes(app, ui);
                Self::render_signature_section(app, ui);
                Self::render_draft_management_section(app, ui);

                ui.separator();

                ui.vertical_centered_justified(|ui| {
                    if ui.button("🚀 Confirm & Export PDF").clicked() {
                        should_close = Self::handle_confirm_export_pdf(app);
                    }
                });
            });

        app.show_export_wizard = open && !should_close;
    }

    fn render_compliance_checkboxes(app: &mut crate::app::FerruginousApp, ui: &mut egui::Ui) {
        ui.heading("Export & Compliance Options");
        ui.add_space(5.0);

        ui.checkbox(&mut app.export_upgrade_pdf20, "Upgrade to PDF 2.0 (ISO 32000-2)");
        ui.checkbox(&mut app.export_linearize, "Hint Table Linearization (Fast Web View)");
        ui.checkbox(&mut app.export_vacuum, "Vacuum Pass (Remove orphan/unreachable objects)");
        ui.checkbox(&mut app.export_compress, "Flate-Compress Content Streams");
        ui.checkbox(&mut app.export_apply_tags, "Compile & Inject USTRegistry Tags (PDF/UA-2)");
        ui.checkbox(&mut app.export_burn_redactions, "Burn Physical Redactions (Atomic Stream Sanitization)");
    }

    fn render_signature_section(app: &mut crate::app::FerruginousApp, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("Digital Signature (PAdES)");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.button("Select Certificate (.pfx/.p12)").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("PKCS#12", &["pfx", "p12"])
                    .pick_file()
                {
                    app.cert_path = Some(p);
                }
            }
            if let Some(path) = &app.cert_path {
                ui.label(path.file_name().unwrap_or(&path.as_os_str()).to_string_lossy());
            } else {
                ui.label("No certificate loaded");
            }
        });

        if app.cert_path.is_some() {
            ui.horizontal(|ui| {
                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut app.cert_password).password(true));
            });

            ui.horizontal(|ui| {
                if ui.toggle_value(&mut app.is_placing_signature, "Place Signature Field").clicked() {
                    if app.is_placing_signature {
                        app.show_export_wizard = false;
                    }
                }
                if let Some((page, rect)) = &app.signature_position {
                    ui.label(format!("Placed: Page {}, Pos ({:.1}, {:.1})", page + 1, rect.min.x, rect.min.y));
                } else {
                    ui.label("Not placed yet");
                }
            });
        }
    }

    fn render_draft_management_section(app: &mut crate::app::FerruginousApp, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("Draft Management");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.button("Save UST Draft JSON").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("ust_draft.json")
                    .save_file()
                {
                    if let Ok(json_str) = serde_json::to_string_pretty(&app.ust_registry) {
                        if std::fs::write(&p, json_str).is_ok() {
                            app.error = Some(format!("Draft JSON saved to {:?}", p.file_name().unwrap_or(&p.as_os_str())));
                        } else {
                            app.error = Some("Failed to write draft JSON file".to_string());
                        }
                    }
                }
            }

            if ui.button("Load UST Draft JSON").clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    if let Ok(bytes) = std::fs::read(&p) {
                        if let Ok(draft) = serde_json::from_slice::<USTRegistry>(&bytes) {
                            app.ust_registry = draft;
                            app.error = Some("Draft JSON loaded successfully".to_string());
                        } else {
                            app.error = Some("Invalid draft JSON structure".to_string());
                        }
                    }
                }
            }
        });
    }

    fn handle_confirm_export_pdf(app: &mut crate::app::FerruginousApp) -> bool {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("output_compliant.pdf")
            .save_file()
        {
            if app.export_burn_redactions {
                let mut keys: Vec<usize> = app.raw_texts.keys().cloned().collect();
                keys.sort();
                for page_idx in keys {
                    if let (Some(raw_text), Some(spans)) = (
                        app.raw_texts.get(&page_idx).cloned(),
                        app.page_spans.get_mut(&page_idx),
                    ) {
                        let sanitized = app.redaction_manager.perform_physical_redaction(
                            page_idx,
                            &raw_text,
                            spans,
                        );
                        app.raw_texts.insert(page_idx, sanitized);
                    }
                }
                app.redaction_manager.clear();
            }

            let sig_pos = app.signature_position.map(|(idx, r)| {
                (idx, [r.min.x, r.min.y, r.max.x, r.max.y])
            });
            let _ = app.tx_worker.send(WorkerRequest::Save {
                path: p,
                compress: app.export_compress,
                linearize: app.export_linearize,
                vacuum: app.export_vacuum,
                upgrade_pdf20: app.export_upgrade_pdf20,
                redaction_zones: app.redaction_manager.zones.clone(),
                cert_path: app.cert_path.clone(),
                cert_password: app.cert_password.clone(),
                signature_position: sig_pos,
            });
            true
        } else {
            false
        }
    }
}
