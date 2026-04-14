use eframe::egui;
use egui::{Color32, RichText, Frame, Margin, Stroke};
use crate::FerruginousApp;
use ferruginous_sdk::core::Reference;
use ferruginous_sdk::navigation::{Outline, OutlineItem, Destination};

/// Renders the minimalist side panel.
pub fn show_sidebar(app: &mut FerruginousApp, ctx: &egui::Context) {
    let bg_fill = Color32::from_rgb(252, 252, 253);
    let border_color = Color32::from_rgb(235, 235, 240);

    egui::SidePanel::left("sidebar_premium")
        .default_width(260.0)
        .frame(Frame::default()
            .fill(bg_fill)
            .stroke(Stroke::new(1.0, border_color))
            .inner_margin(Margin::symmetric(20, 24)))
        .show(ctx, |ui| {
            show_sidebar_tabs(app, ui);
            ui.add_space(24.0);

            egui::ScrollArea::vertical()
                .id_salt("sidebar_scroll")
                .show(ui, |ui| {
                    match app.active_tab.as_str() {
                        "Page" => show_pages_tab(app, ui),
                        "Outline" => show_outline_tab(app, ui),
                        "Layer" => show_layers_tab(app, ui),
                        "Search" => show_search_tab(app, ui),
                        _ => {}
                    }
                });
        });
}

fn show_sidebar_tabs(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    let rust = Color32::from_rgb(183, 65, 14);
    ui.horizontal(|ui| {
        for name in &["Page", "Outline", "Layer", "Search"] {
            let is_active = app.active_tab == *name;
            let text = RichText::new(*name).strong();
            let resp = ui.selectable_label(is_active, text);
            
            if resp.clicked() {
                app.active_tab = name.to_string();
            }

            if is_active {
                let rect = resp.rect;
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(2.0, rust)
                );
            }
        }
    });
}

fn show_pages_tab(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Page List").strong().size(14.0));
    ui.add_space(12.0);
    
    if app.page_count == 0 {
        ui.label(RichText::new("No document loaded").weak());
    } else {
        for i in 0..app.page_count {
            let is_current = app.current_page == i;
            if ui.selectable_label(is_current, format!("Page {}", i + 1)).clicked() {
                app.current_page = i;
                app.update_rendering();
            }
        }
    }

    ui.add_space(32.0);
    ui.separator();
    ui.add_space(12.0);
    ui.label(RichText::new("System & Debug").strong().color(Color32::from_rgb(183, 65, 14)));
    ui.label(format!("PDF Loaded: {}", app.pdf_doc.is_some()));
    ui.label(format!("Page Count: {}", app.page_count));
    ui.label(format!("Draw Ops: {}", app.last_draw_op_count));
    ui.label(format!("Render Count: {}", app.vello_callback_count));
    ui.label(format!("Texture ID: {:?}", app.vello_texture_id));
    ui.label(format!("GPU: {}", app.gpu_name));
    if let Some(err) = &app.vello_init_error {
        ui.label(RichText::new(format!("Vello Error: {err}")).color(Color32::RED));
    }
}

fn show_outline_tab(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Document Outline").strong().size(14.0));
    ui.add_space(12.0);
    
    let mut nav_target = None;
    if let Some(doc) = &app.pdf_doc {
        if let Ok(catalog) = doc.catalog() {
            if let Some(outline) = catalog.outlines() {
                nav_target = render_outline_tree(ui, outline);
            } else {
                ui.label(RichText::new("No outline information available").weak());
            }
        }
    } else {
        ui.label(RichText::new("Please load a PDF").weak());
    }

    if let Some(target_ref) = nav_target {
        if let Some(doc) = &app.pdf_doc {
            if let Ok(tree) = doc.page_tree() {
                if let Some(idx) = tree.find_page_index(&target_ref) {
                    app.current_page = idx;
                    app.update_rendering();
                }
            }
        }
    }
}

fn render_outline_tree(ui: &mut egui::Ui, outline: Outline) -> Option<Reference> {
    let mut clicked_ref = None;
    if let Some(first_ref) = outline.first() {
        let mut stack = vec![(first_ref, 0)];
        while let Some((current_ref, depth)) = stack.pop() {
            if let Ok(ferruginous_sdk::core::Object::Dictionary(dict)) = outline.resolver.resolve(&current_ref) {
                let item = OutlineItem::new(dict, current_ref, outline.resolver);
                let title = item.title().unwrap_or_else(|| "Untitled".to_string());
                
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 12.0);
                    if ui.selectable_label(false, title).clicked() {
                        if let Some(Destination::Explicit { page, .. }) = item.destination() {
                            clicked_ref = Some(page);
                        }
                    }
                });

                if let Some(next_ref) = item.next() { stack.push((next_ref, depth)); }
                if let Some(child_ref) = item.first_child() { stack.push((child_ref, depth + 1)); }
            }
        }
    }
    clicked_ref
}

fn show_layers_tab(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Visible Layers").strong().size(14.0));
    ui.add_space(12.0);
    
    if let Some(ref mut ctx) = app.oc_context {
        let mut changed = false;
        for ocg in &app.available_ocgs {
            let mut is_on = *ctx.states.get(&ocg.reference).unwrap_or(&true);
            let label = String::from_utf8_lossy(&ocg.name).to_string();
            if ui.checkbox(&mut is_on, label).changed() {
                ctx.states.insert(ocg.reference, is_on);
                changed = true;
            }
        }
        if changed { app.update_rendering(); }
    } else {
        ui.label(RichText::new("No layer information available").weak());
    }
}

fn show_search_tab(app: &mut FerruginousApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Text Search").strong().size(14.0));
    ui.add_space(12.0);
    
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut app.search_query);
        if ui.button("Search").clicked() {
            // Execution of search: Logic remains to be integrated if needed
        }
    });
}
