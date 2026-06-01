use crate::interaction::{SelectionManager, TextSpan};
use crate::redaction::RedactionManager;
use crate::sidebar::{SidebarPanel, USTRegistry};
use crate::vello_egui::VelloRenderer;
use crate::view::{PDFView, PageLayout};
use crate::worker::{WorkerRequest, WorkerResponse, run_worker};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use vello::Scene;

pub struct FerruginousApp {
    tx_worker: Sender<WorkerRequest>,
    rx_worker: Receiver<WorkerResponse>,

    total_pages: usize,
    page_layouts: Vec<PageLayout>,

    view: PDFView,
    error: Option<String>,
    pdf_name: Option<String>,

    vello_renderer: Option<VelloRenderer>,
    scenes: BTreeMap<usize, Arc<Scene>>,
    request_queue: BTreeSet<usize>,

    selection_manager: SelectionManager,
    page_spans: BTreeMap<usize, Vec<TextSpan>>,

    ust_registry: USTRegistry,
    sidebar_panel: SidebarPanel,

    redaction_manager: RedactionManager,
    redaction_studio_panel: crate::redaction_studio::RedactionStudioPanel,
    show_export_wizard: bool,
    export_compress: bool,
    export_linearize: bool,
    export_vacuum: bool,
    export_upgrade_pdf20: bool,
    export_apply_tags: bool,
    export_burn_redactions: bool,
    raw_texts: BTreeMap<usize, String>, // page_index -> raw extracted text

    // Digital Signature & Placement
    pub cert_path: Option<PathBuf>,
    pub cert_password: String,
    pub signature_position: Option<(usize, egui::Rect)>, // (page_index, rect in PDF user space)
    pub is_placing_signature: bool,

    // CAD snappers & Inspector
    pub cad_snap_engine: crate::cad_canvas::CadSnapEngine,
    pub caliper_tool: crate::cad_canvas::CaliperTool,
    pub arlington_inspector: crate::inspector::ArlingtonInspectorPanel,
    pub show_inspector: bool,
}

impl FerruginousApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let vello_renderer =
            cc.wgpu_render_state.as_ref().and_then(|rs| VelloRenderer::new(&rs.device));
        let (tx_req, rx_req) = channel();
        let (tx_res, rx_res) = channel();

        std::thread::spawn(move || {
            run_worker(rx_req, tx_res);
        });

        Self {
            tx_worker: tx_req,
            rx_worker: rx_res,
            total_pages: 0,
            page_layouts: Vec::new(),
            view: PDFView::new(),
            error: None,
            pdf_name: None,
            vello_renderer,
            scenes: BTreeMap::new(),
            request_queue: BTreeSet::new(),
            selection_manager: SelectionManager::new(),
            page_spans: BTreeMap::new(),
            ust_registry: USTRegistry::new(),
            sidebar_panel: SidebarPanel::new(),
            redaction_manager: RedactionManager::new(),
            redaction_studio_panel: crate::redaction_studio::RedactionStudioPanel::new(),
            show_export_wizard: false,
            export_compress: true,
            export_linearize: true,
            export_vacuum: true,
            export_upgrade_pdf20: true,
            export_apply_tags: true,
            export_burn_redactions: true,
            raw_texts: BTreeMap::new(),

            // Signature Defaults
            cert_path: None,
            cert_password: String::new(),
            signature_position: None,
            is_placing_signature: false,

            // CAD & Inspector Defaults
            cad_snap_engine: crate::cad_canvas::CadSnapEngine::new(),
            caliper_tool: crate::cad_canvas::CaliperTool::new(),
            arlington_inspector: crate::inspector::ArlingtonInspectorPanel::new(),
            show_inspector: false,
        }
    }

    fn inject_tag_to_tree(&mut self, tag: &str, req: &crate::interaction::PendingTagRequest) {
        let new_node = crate::sidebar::USTNode {
            id: self.ust_registry.next_node_id,
            tag: tag.to_string(),
            title: if req.text.len() > 30 { format!("{}...", &req.text[..30]) } else { req.text.clone() },
            alt_text: if tag == "Figure" { Some(req.text.clone()) } else { None },
            rect: Some([req.combined_rect.min.x, req.combined_rect.min.y, req.combined_rect.max.x, req.combined_rect.max.y]),
            handle_id: None,
            children: Vec::new(),
        };
        self.ust_registry.next_node_id += 1;

        if let Some(ref mut root) = self.ust_registry.root {
            root.children.push(new_node);
        }

        self.error = Some(format!("Successfully created <{}> tag", tag));
    }

    pub fn open_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        if let Ok(bytes) = std::fs::read(&path) {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            self.open_file_bytes(bytes::Bytes::from(bytes), name, ctx);
        }
    }

    pub fn open_file_bytes(&mut self, data: bytes::Bytes, name: Option<String>, _ctx: &egui::Context) {
        self.error = None;
        self.total_pages = 0;
        self.page_layouts.clear();
        self.scenes.clear();
        self.request_queue.clear();
        self.selection_manager.clear();
        self.page_spans.clear();
        self.ust_registry.clear();
        self.reset_view();
        let _ = self.tx_worker.send(WorkerRequest::Open { data, name });
    }

    fn reset_view(&mut self) {
        self.view.zoom = 1.0;
        self.view.pan = egui::Vec2::ZERO;
    }

    fn process_worker_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx_worker.try_recv() {
            match msg {
                WorkerResponse::DocumentLoaded {
                    name,
                    num_pages,
                    page_sizes,
                    page_texts,
                    ust_root,
                    audit_findings,
                } => {
                    self.pdf_name = name;
                    self.total_pages = num_pages;
                    self.compute_layouts(&page_sizes);

                    // Pre-generate TextSpans for each page
                    for (i, text) in page_texts.iter().enumerate() {
                        let size = page_sizes.get(i).cloned().unwrap_or((595.0, 842.0));
                        let spans = SelectionManager::generate_spans_for_page(text, size.0 as f32, size.1 as f32);
                        self.page_spans.insert(i, spans);
                        self.raw_texts.insert(i, text.clone());
                    }

                    // Load parsed accessibility tag tree & real Matterhorn audit findings
                    self.ust_registry.root = ust_root;
                    self.ust_registry.audit_findings = audit_findings;

                    ctx.request_repaint();
                }
                WorkerResponse::PageRendered { index, scene, .. } => {
                    self.scenes.insert(index, scene);
                    self.request_queue.remove(&index);
                    ctx.request_repaint();
                }
                WorkerResponse::AuditFindings { findings } => {
                    self.ust_registry.audit_findings = findings;
                    ctx.request_repaint();
                }
                WorkerResponse::DocumentSaved { path } => {
                    self.error = Some(format!(
                        "Successfully exported compliant PDF to {:?}",
                        path.file_name().unwrap_or(&path.as_os_str())
                    ));
                    self.show_export_wizard = false;
                    ctx.request_repaint();
                }
                WorkerResponse::Error(err) => {
                    self.error = Some(err);
                }
            }
        }
    }

    fn compute_layouts(&mut self, page_sizes: &[(f64, f64)]) {
        let mut layouts = Vec::with_capacity(page_sizes.len());
        let mut current_y = 0.0;
        let gap = 20.0;

        for (i, &(w, h)) in page_sizes.iter().enumerate() {
            let w = w as f32;
            let h = h as f32;
            let x = -w / 2.0;
            let rect = egui::Rect::from_min_size(egui::pos2(x, current_y), egui::vec2(w, h));
            layouts.push(PageLayout { index: i, rect });
            current_y += h + gap;
        }
        self.page_layouts = layouts;
    }

    fn check_gpu_support(&self, ui: &mut egui::Ui, frame: &mut eframe::Frame) -> bool {
        let has_wgpu = frame.wgpu_render_state().is_some();
        let has_vello = self.vello_renderer.is_some();
        if !has_wgpu || !has_vello {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.colored_label(egui::Color32::RED, "🚨 GPU Compute Support Error");
                    ui.add_space(20.0);
                    ui.label("Ferruginous relies on high-performance GPU Compute Shaders (Vello) to render PDFs.");
                    ui.label("Either WebGPU is not supported by your hardware/browser, or your GPU drivers do not support compute shaders.");
                    ui.add_space(20.0);
                    ui.label("Please ensure WebGPU or WebGL2-Compute support is enabled, and your graphics drivers are up to date.");
                });
            });
            false
        } else {
            true
        }
    }

    fn queue_visible_pages(&mut self) {
        // Collect visible pages and calculate pre-render lookahead indices
        let mut render_targets = std::collections::BTreeSet::new();
        for &visible_index in &self.view.visible_pages {
            render_targets.insert(visible_index);

            // Lookahead pre-rendering: queue previous page and next page in the background
            if visible_index > 0 {
                render_targets.insert(visible_index - 1);
            }
            if visible_index + 1 < self.total_pages {
                render_targets.insert(visible_index + 1);
            }
        }

        // Queue rendering requests to the worker thread
        for index in render_targets {
            if !self.scenes.contains_key(&index) && !self.request_queue.contains(&index) {
                let scale = 2.0;
                self.request_queue.insert(index);
                let _ = self.tx_worker.send(WorkerRequest::RenderPage { index, scale });
            }
        }
    }

    fn handle_signature_placement_interaction(
        &mut self,
        ui: &mut egui::Ui,
        visible_index: usize,
        page_screen_rect: egui::Rect,
        unscaled_h: f32,
        zoom: f32,
    ) {
        let response = ui.allocate_rect(page_screen_rect, egui::Sense::drag());
        let screen_pos = ui.input(|i| i.pointer.hover_pos());

        if response.drag_started() && let Some(pos) = screen_pos {
            let pdf_pos = SelectionManager::screen_to_pdf(page_screen_rect, zoom, unscaled_h, pos);
            self.signature_position = Some((visible_index, egui::Rect::from_min_max(pdf_pos, pdf_pos)));
        }

        if response.dragged() && let Some(pos) = screen_pos && let Some((sig_idx, sig_rect)) = &mut self.signature_position {
            if *sig_idx == visible_index {
                let pdf_pos = SelectionManager::screen_to_pdf(page_screen_rect, zoom, unscaled_h, pos);
                let start_pos = sig_rect.min;
                *sig_rect = egui::Rect::from_two_pos(start_pos, pdf_pos);
            }
        }

        if response.drag_stopped() {
            self.is_placing_signature = false;
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
    }

    fn handle_page_interactions(&mut self, ui: &mut egui::Ui, viewport_rect: egui::Rect, zoom: f32) { // RR-15 Limit: GUI - Unified egui pointer and canvas coordinate interaction loop
        let visible_pages = self.view.visible_pages.clone();
        for &visible_index in &visible_pages {
            if let Some(layout) = self.page_layouts.get(visible_index) {
                let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;
                let page_screen_rect = egui::Rect::from_min_size(
                    origin + layout.rect.min.to_vec2() * zoom,
                    layout.rect.size() * zoom,
                );
                let unscaled_h = layout.rect.height();

                if self.is_placing_signature {
                    self.handle_signature_placement_interaction(ui, visible_index, page_screen_rect, unscaled_h, zoom);
                } else if self.caliper_tool.is_active {
                    if let Some(spans) = self.page_spans.get(&visible_index) {
                        self.caliper_tool.handle_interaction(
                            ui,
                            visible_index,
                            page_screen_rect,
                            unscaled_h,
                            zoom,
                            &mut self.cad_snap_engine,
                            spans,
                        );
                        self.caliper_tool.draw_overlay(ui, page_screen_rect, unscaled_h, zoom);
                    }
                } else if self.redaction_manager.is_active {
                    self.redaction_manager.handle_interaction(
                        ui,
                        visible_index,
                        page_screen_rect,
                        unscaled_h,
                        zoom,
                    );
                } else if let Some(spans) = self.page_spans.get(&visible_index) {
                    if self.selection_manager.is_tagging_brush_active {
                        self.selection_manager.handle_tagging_brush_interaction(
                            ui,
                            visible_index,
                            page_screen_rect,
                            unscaled_h,
                            spans,
                            zoom,
                        );
                    } else {
                        self.selection_manager.handle_interaction(
                            ui,
                            visible_index,
                            page_screen_rect,
                            unscaled_h,
                            spans,
                            zoom,
                        );
                    }
                }
            }
        }
    }

    fn get_structural_highlight(&self, viewport_rect: egui::Rect, zoom: f32) -> Option<(usize, egui::Rect)> {
        let selected_id = self.ust_registry.selected_node_id?;
        if let Some(ref root) = self.ust_registry.root {
            if root.id == selected_id {
                return None;
            }
        }
        let rect = self.ust_registry.find_rect_by_id(selected_id)?;
        let page_idx = 0; // Default to first page
        let layout = self.page_layouts.get(page_idx)?;
        let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;
        let page_screen_rect = egui::Rect::from_min_size(
            origin + layout.rect.min.to_vec2() * zoom,
            layout.rect.size() * zoom,
        );
        let unscaled_h = layout.rect.height();
        let screen_min = SelectionManager::pdf_to_screen(
            page_screen_rect,
            zoom,
            unscaled_h,
            egui::pos2(rect[0], rect[3]),
        );
        let screen_max = SelectionManager::pdf_to_screen(
            page_screen_rect,
            zoom,
            unscaled_h,
            egui::pos2(rect[2], rect[1]),
        );
        Some((page_idx, egui::Rect::from_min_max(screen_min, screen_max)))
    }

    fn get_signature_highlight(&self, viewport_rect: egui::Rect, zoom: f32) -> Option<(usize, egui::Rect)> {
        let (sig_page, sig_rect) = self.signature_position?;
        let layout = self.page_layouts.get(sig_page)?;
        let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;
        let page_screen_rect = egui::Rect::from_min_size(
            origin + layout.rect.min.to_vec2() * zoom,
            layout.rect.size() * zoom,
        );
        let unscaled_h = layout.rect.height();
        let screen_min = SelectionManager::pdf_to_screen(
            page_screen_rect,
            zoom,
            unscaled_h,
            egui::pos2(sig_rect.min.x, sig_rect.max.y),
        );
        let screen_max = SelectionManager::pdf_to_screen(
            page_screen_rect,
            zoom,
            unscaled_h,
            egui::pos2(sig_rect.max.x, sig_rect.min.y),
        );
        Some((sig_page, egui::Rect::from_min_max(screen_min, screen_max)))
    }

    fn draw_view_with_highlights(
        &mut self,
        ui: &mut egui::Ui,
        viewport_rect: egui::Rect,
        zoom: f32,
        viewport_texture_id: Option<egui::TextureId>,
    ) {
        // Gather redaction highlights
        let mut redaction_highlights = BTreeMap::new();
        let mut active_redaction_drag = None;

        let visible_pages = self.view.visible_pages.clone();
        for &visible_index in &visible_pages {
            if let Some(layout) = self.page_layouts.get(visible_index) {
                let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;
                let page_screen_rect = egui::Rect::from_min_size(
                    origin + layout.rect.min.to_vec2() * zoom,
                    layout.rect.size() * zoom,
                );
                let unscaled_h = layout.rect.height();

                let (completed, active_drag) = self.redaction_manager.get_screen_highlights(
                    visible_index,
                    page_screen_rect,
                    unscaled_h,
                    zoom,
                );
                if !completed.is_empty() {
                    redaction_highlights.insert(visible_index, completed);
                }
                if let Some(drag_rect) = active_drag {
                    active_redaction_drag = Some((visible_index, drag_rect));
                }
            }
        }

        let structural_highlight = self.get_structural_highlight(viewport_rect, zoom);
        let signature_highlight = self.get_signature_highlight(viewport_rect, zoom);

        self.view.show_virtual(
            ui,
            &self.page_layouts,
            viewport_texture_id,
            viewport_rect,
            &self.scenes,
            &self.selection_manager.highlights,
            &redaction_highlights,
            &active_redaction_drag,
            &structural_highlight,
            &signature_highlight,
        );
    }

    fn render_document_panel(
        &mut self,
        ui: &mut egui::Ui,
        rs: &egui_wgpu::RenderState,
        viewport_rect: egui::Rect,
    ) {
        if let Some(center_id) = self.ust_registry.pending_center_node_id.take() {
            if let Some(rect) = self.ust_registry.find_rect_by_id(center_id) {
                let page_idx = 0; // Default to first page
                if let Some(layout) = self.page_layouts.get(page_idx) {
                    self.view.center_on_rect(viewport_rect, layout, rect);
                }
            }
        }

        let vello_renderer = match self.vello_renderer.as_mut() {
            Some(r) => r,
            None => return,
        };
        vello_renderer.next_frame(rs);

        let zoom = self.view.zoom;

        // Collect visible pages and their scenes
        let mut visible_pages_data = Vec::new();
        let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;

        for layout in &self.page_layouts {
            let page_screen_rect = egui::Rect::from_min_size(
                origin + layout.rect.min.to_vec2() * zoom,
                layout.rect.size() * zoom,
            );

            if viewport_rect.intersects(page_screen_rect) {
                if let Some(scene) = self.scenes.get(&layout.index) {
                    let unscaled_size = egui::vec2(layout.rect.width(), layout.rect.height());
                    visible_pages_data.push((layout.index, Arc::clone(scene), page_screen_rect, unscaled_size));
                }
            }
        }

        let scale_factor = ui.ctx().pixels_per_point();
        let viewport_texture_id = vello_renderer.render_viewport(
            rs,
            &visible_pages_data,
            viewport_rect,
            scale_factor,
            zoom,
        );

        self.handle_page_interactions(ui, viewport_rect, zoom);
        self.draw_view_with_highlights(ui, viewport_rect, zoom, viewport_texture_id);
    }

    fn update_vello(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.process_worker_messages(&ctx);

        if !self.check_gpu_support(ui, frame) {
            return;
        }

        let rs = match frame.wgpu_render_state() {
            Some(state) => state,
            None => return,
        };

        self.queue_visible_pages();

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(err) = &self.error {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::RED, err);
                });
            } else if !self.page_layouts.is_empty() {
                let viewport_rect = ui.clip_rect();
                self.render_document_panel(ui, rs, viewport_rect);
            } else if self.pdf_name.is_some() {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading document...");
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("Ferruginous");
                    ui.label("Fast, Secure, GPU-Accelerated PDF Viewer");
                    ui.add_space(20.0);
                    if ui.button("Open a PDF").clicked()
                        && let Some(p) =
                            rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file()
                    {
                        self.open_file(p, &ctx);
                    }
                });
            }
        });
    }

    fn show_top_bar(&mut self, ui: &mut egui::Ui) { // RR-15 Limit: GUI - Sequential egui top menu bar layout routing view resets, zoom, redact brush, tagging brush, caliper tool, and save wizard
        let ctx = ui.ctx().clone();
        egui::Panel::top("top_bar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Open PDF").clicked()
                    && let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file()
                {
                    self.open_file(p, &ctx);
                }
                ui.separator();
                if self.total_pages > 0 {
                    ui.label(format!("{} Pages", self.total_pages));
                    if ui.button("Reset View").clicked() {
                        self.reset_view();
                    }
                    ui.separator();
                    ui.label(format!("Zoom: {:.1}%", self.view.zoom * 100.0));
                    if let Some(name) = &self.pdf_name {
                        ui.separator();
                        ui.label(name);
                    }
                    ui.separator();

                    // Redact Brush toggle
                    let active_before = self.redaction_manager.is_active;
                    ui.toggle_value(&mut self.redaction_manager.is_active, "🔏 Redact Brush");
                    if self.redaction_manager.is_active && !active_before {
                        self.selection_manager.clear();
                        self.selection_manager.is_tagging_brush_active = false;
                        self.caliper_tool.is_active = false;
                    }

                    ui.separator();

                    // Tagging Brush toggle
                    let tagging_before = self.selection_manager.is_tagging_brush_active;
                    ui.toggle_value(&mut self.selection_manager.is_tagging_brush_active, "🏷️ Tagging Brush");
                    if self.selection_manager.is_tagging_brush_active && !tagging_before {
                        self.selection_manager.clear();
                        self.redaction_manager.is_active = false;
                        self.caliper_tool.is_active = false;
                    }

                    ui.separator();

                    // Caliper Brush toggle
                    let caliper_before = self.caliper_tool.is_active;
                    ui.toggle_value(&mut self.caliper_tool.is_active, "📏 Caliper Brush");
                    if self.caliper_tool.is_active && !caliper_before {
                        self.selection_manager.clear();
                        self.redaction_manager.is_active = false;
                        self.selection_manager.is_tagging_brush_active = false;
                    }

                    ui.separator();

                    // Arlington Inspector toggle
                    ui.toggle_value(&mut self.show_inspector, "🔍 Inspector");

                    ui.separator();
                    if ui.button("💾 Export PDF").clicked() {
                        self.show_export_wizard = true;
                    }
                }
            });
        });
    }

    fn render_compliance_checkboxes(&mut self, ui: &mut egui::Ui) {
        ui.heading("Export & Compliance Options");
        ui.add_space(5.0);

        ui.checkbox(&mut self.export_upgrade_pdf20, "✨ Upgrade to PDF 2.0 (ISO 32000-2)");
        ui.checkbox(&mut self.export_linearize, "⚡ Hint Table Linearization (Fast Web View)");
        ui.checkbox(&mut self.export_vacuum, "🧹 Vacuum Pass (Remove orphan/unreachable objects)");
        ui.checkbox(&mut self.export_compress, "📦 Flate-Compress Content Streams");
        ui.checkbox(&mut self.export_apply_tags, "🏷️ Compile & Inject USTRegistry Tags (PDF/UA-2)");
        ui.checkbox(&mut self.export_burn_redactions, "🔏 Burn Physical Redactions (Atomic Stream Sanitization)");
    }

    fn render_signature_section(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("🔏 Digital Signature (PAdES)");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.button("📁 Select Certificate (.pfx/.p12)").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("PKCS#12", &["pfx", "p12"])
                    .pick_file()
                {
                    self.cert_path = Some(p);
                }
            }
            if let Some(path) = &self.cert_path {
                ui.label(path.file_name().unwrap_or(&path.as_os_str()).to_string_lossy());
            } else {
                ui.label("No certificate loaded");
            }
        });

        if self.cert_path.is_some() {
            ui.horizontal(|ui| {
                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut self.cert_password).password(true));
            });

            ui.horizontal(|ui| {
                if ui.toggle_value(&mut self.is_placing_signature, "🎯 Place Signature Field").clicked() {
                    if self.is_placing_signature {
                        self.show_export_wizard = false;
                    }
                }
                if let Some((page, rect)) = &self.signature_position {
                    ui.label(format!("Placed: Page {}, Pos ({:.1}, {:.1})", page + 1, rect.min.x, rect.min.y));
                } else {
                    ui.label("Not placed yet");
                }
            });
        }
    }

    fn render_draft_management_section(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("Draft Management");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.button("📥 Save UST Draft JSON").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("ust_draft.json")
                    .save_file()
                {
                    if let Ok(json_str) = serde_json::to_string_pretty(&self.ust_registry) {
                        if std::fs::write(&p, json_str).is_ok() {
                            self.error = Some(format!("Draft JSON saved to {:?}", p.file_name().unwrap_or(&p.as_os_str())));
                        } else {
                            self.error = Some("Failed to write draft JSON file".to_string());
                        }
                    }
                }
            }

            if ui.button("📤 Load UST Draft JSON").clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    if let Ok(bytes) = std::fs::read(&p) {
                        if let Ok(draft) = serde_json::from_slice::<USTRegistry>(&bytes) {
                            self.ust_registry = draft;
                            self.error = Some("Draft JSON loaded successfully".to_string());
                        } else {
                            self.error = Some("Invalid draft JSON structure".to_string());
                        }
                    }
                }
            }
        });
    }

    fn handle_confirm_export_pdf(&mut self) -> bool {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("output_compliant.pdf")
            .save_file()
        {
            if self.export_burn_redactions {
                let mut keys: Vec<usize> = self.raw_texts.keys().cloned().collect();
                keys.sort();
                for page_idx in keys {
                    if let (Some(raw_text), Some(spans)) = (
                        self.raw_texts.get(&page_idx).cloned(),
                        self.page_spans.get_mut(&page_idx),
                    ) {
                        let sanitized = self.redaction_manager.perform_physical_redaction(
                            page_idx,
                            &raw_text,
                            spans,
                        );
                        self.raw_texts.insert(page_idx, sanitized);
                    }
                }
                self.redaction_manager.clear();
            }

            let sig_pos = self.signature_position.map(|(idx, r)| {
                (idx, [r.min.x, r.min.y, r.max.x, r.max.y])
            });
            let _ = self.tx_worker.send(WorkerRequest::Save {
                path: p,
                compress: self.export_compress,
                linearize: self.export_linearize,
                vacuum: self.export_vacuum,
                upgrade_pdf20: self.export_upgrade_pdf20,
                redaction_zones: self.redaction_manager.zones.clone(),
                cert_path: self.cert_path.clone(),
                cert_password: self.cert_password.clone(),
                signature_position: sig_pos,
            });
            true
        } else {
            false
        }
    }

    fn show_export_wizard_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_export_wizard;
        let mut should_close = false;
        egui::Window::new("💾 Production Studio Export Wizard")
            .open(&mut open)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                self.render_compliance_checkboxes(ui);
                self.render_signature_section(ui);
                self.render_draft_management_section(ui);

                ui.separator();

                ui.vertical_centered_justified(|ui| {
                    if ui.button("🚀 Confirm & Export PDF").clicked() {
                        should_close = self.handle_confirm_export_pdf();
                    }
                });
            });
        self.show_export_wizard = open && !should_close;
    }
}

impl eframe::App for FerruginousApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) { // RR-15 Limit: GUI - Main application UI shell layout routing layout panels and windows
        self.show_top_bar(ui);

        if self.total_pages > 0 {
            egui::Panel::left("left_sidebar")
                .resizable(true)
                .default_size(320.0)
                .size_range(200.0..=500.0)
                .show_inside(ui, |ui| {
                    self.sidebar_panel.show(ui, &mut self.ust_registry, &self.tx_worker);
                });

            egui::Panel::right("right_sidebar")
                .resizable(true)
                .default_size(320.0)
                .size_range(200.0..=500.0)
                .show_inside(ui, |ui| {
                    self.redaction_studio_panel.show(
                        ui,
                        &self.raw_texts,
                        &self.page_spans,
                        &mut self.redaction_manager,
                    );
                });

            if self.show_inspector {
                let selected_tag = self.ust_registry.selected_node_id
                    .and_then(|id| self.ust_registry.root.as_ref()
                        .and_then(|r| crate::sidebar::USTRegistry::find_node_by_id_recursive(r, id))
                        .map(|n| n.tag.as_str())
                    );
                egui::Panel::bottom("inspector_panel")
                    .resizable(true)
                    .default_size(220.0)
                    .show_inside(ui, |ui| {
                        self.arlington_inspector.show(ui, selected_tag);
                    });
            }
        }

        self.update_vello(ui, frame);

        if self.show_export_wizard {
            self.show_export_wizard_window(ui.ctx());
        }

        // Show interactive Create Semantic Tag popup dialog on visual tag selector brush highlights
        if let Some(req) = self.selection_manager.pending_tag_request.clone() {
            let mut show_popup = true;
            egui::Window::new("🏷️ Create Semantic Tag")
                .open(&mut show_popup)
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Selected Text:");
                    ui.group(|ui| {
                        ui.label(&req.text);
                    });
                    ui.add_space(5.0);
                    ui.label("Select tag level to assign to structure tree:");

                    ui.horizontal(|ui| {
                        if ui.button("H1").clicked() {
                            self.inject_tag_to_tree("H1", &req);
                            self.selection_manager.pending_tag_request = None;
                        }
                        if ui.button("H2").clicked() {
                            self.inject_tag_to_tree("H2", &req);
                            self.selection_manager.pending_tag_request = None;
                        }
                        if ui.button("P").clicked() {
                            self.inject_tag_to_tree("P", &req);
                            self.selection_manager.pending_tag_request = None;
                        }
                        if ui.button("Figure").clicked() {
                            self.inject_tag_to_tree("Figure", &req);
                            self.selection_manager.pending_tag_request = None;
                        }
                    });

                    if ui.button("Cancel").clicked() {
                        self.selection_manager.pending_tag_request = None;
                    }
                });
            if !show_popup {
                self.selection_manager.pending_tag_request = None;
            }
        }
    }
}
