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
    pub tx_worker: Sender<WorkerRequest>,
    pub rx_worker: Receiver<WorkerResponse>,

    pub total_pages: usize,
    pub page_layouts: Vec<PageLayout>,

    pub view: PDFView,
    pub error: Option<String>,
    pub pdf_name: Option<String>,

    pub vello_renderer: Option<VelloRenderer>,
    pub scenes: BTreeMap<usize, Arc<Scene>>,
    pub request_queue: BTreeSet<usize>,

    pub selection_manager: SelectionManager,
    pub page_spans: BTreeMap<usize, Vec<TextSpan>>,

    pub ust_registry: USTRegistry,
    pub sidebar_panel: SidebarPanel,

    pub redaction_manager: RedactionManager,
    pub redaction_studio_panel: crate::redaction_studio::RedactionStudioPanel,
    pub show_export_wizard: bool,
    pub export_compress: bool,
    pub export_linearize: bool,
    pub export_vacuum: bool,
    pub export_upgrade_pdf20: bool,
    pub export_apply_tags: bool,
    pub export_burn_redactions: bool,
    pub raw_texts: BTreeMap<usize, String>, // page_index -> raw extracted text

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

    // Selection management
    pub selected_pages: BTreeSet<usize>,
    pub last_selected_page: Option<usize>,
    pub clear_thumbnails_pending: bool,
    pub invalidated_thumbnails: BTreeSet<usize>,
    pub is_loading: bool,
    pub loading_message: String,
    pub show_reading_order: bool,
    pub show_command_palette: bool,
    pub command_palette_search: String,
    pub last_viewport_rect: Option<egui::Rect>,
    pub show_about_modal: bool,
    pub locale_mgr: crate::locale::LocaleManager,
    pub active_language: String,
    pub show_settings_modal: bool,
    pub doc_metadata: Option<ferruginous_core::metadata::MetadataInfo>,
    pub doc_file_size: Option<usize>,
    pub doc_version: Option<String>,
    pub doc_security_method: Option<String>,
    pub doc_permissions: Option<i32>,
    pub doc_page_sizes: Vec<(f64, f64)>,
    pub doc_fonts: Vec<ferruginous_core::font::FontSummary>,
}

impl FerruginousApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // RR-15 Limit: GUI - App state creation and initialization
        let vello_renderer =
            cc.wgpu_render_state.as_ref().and_then(|rs| VelloRenderer::new(&rs.device));
        let (tx_req, rx_req) = channel();
        let (tx_res, rx_res) = channel();

        // Load system CJK/Japanese fonts for egui to support Japanese characters properly
        let mut fonts = egui::FontDefinitions::default();

        // Load Lucide icon font
        let lucide_data = include_bytes!("../assets/lucide.ttf");
        fonts
            .font_data
            .insert("lucide".to_owned(), egui::FontData::from_static(lucide_data).into());
        if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            families.push("lucide".to_owned());
        }
        if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            families.push("lucide".to_owned());
        }

        let paths = [
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ];
        for path in paths {
            if let Ok(font_data) = std::fs::read(path) {
                log::info!("Successfully loaded CJK font from {}", path);
                fonts
                    .font_data
                    .insert("cjk".to_owned(), egui::FontData::from_owned(font_data).into());
                if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    families.insert(0, "cjk".to_owned());
                }
                if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    families.insert(0, "cjk".to_owned());
                }
                break;
            }
        }
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        cc.egui_ctx.global_style_mut(|style| {
            style.visuals.selection.stroke = egui::Stroke::NONE;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_gray(210));
        });

        let egui_ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            run_worker(rx_req, tx_res, egui_ctx);
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

            // Selection Defaults
            selected_pages: BTreeSet::new(),
            last_selected_page: None,
            clear_thumbnails_pending: false,
            invalidated_thumbnails: BTreeSet::new(),
            is_loading: false,
            loading_message: String::new(),
            show_reading_order: true,
            show_command_palette: false,
            command_palette_search: String::new(),
            last_viewport_rect: None,
            show_about_modal: false,
            locale_mgr: crate::locale::LocaleManager::new(),
            active_language: "ja".to_string(), // Defaulting to Japanese since the user requested in Japanese
            show_settings_modal: false,
            doc_metadata: None,
            doc_file_size: None,
            doc_version: None,
            doc_security_method: None,
            doc_permissions: None,
            doc_page_sizes: Vec::new(),
            doc_fonts: Vec::new(),
        }
    }

    fn inject_tag_to_tree(&mut self, tag: &str, req: &crate::interaction::PendingTagRequest) {
        let new_node = crate::sidebar::USTNode {
            id: self.ust_registry.next_node_id,
            tag: tag.to_string(),
            title: if req.text.len() > 30 {
                format!("{}...", &req.text[..30])
            } else {
                req.text.clone()
            },
            alt_text: if tag == "Figure" { Some(req.text.clone()) } else { None },
            rect: Some([
                req.combined_rect.min.x,
                req.combined_rect.min.y,
                req.combined_rect.max.x,
                req.combined_rect.max.y,
            ]),
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

    pub fn open_file_bytes(
        &mut self,
        data: bytes::Bytes,
        name: Option<String>,
        ctx: &egui::Context,
    ) {
        self.error = None;
        self.total_pages = 0;
        self.page_layouts.clear();
        self.scenes.clear();
        self.request_queue.clear();
        self.selection_manager.clear();
        self.page_spans.clear();
        self.ust_registry.clear();
        self.selected_pages.clear();
        self.last_selected_page = None;
        self.clear_thumbnails_pending = true;
        self.is_loading = true;
        self.loading_message = "1/4: Decrypting and normalizing document...".to_string();
        self.doc_metadata = None;
        self.doc_file_size = None;
        self.doc_version = None;
        self.doc_security_method = None;
        self.doc_permissions = None;
        self.doc_page_sizes.clear();
        self.doc_fonts.clear();
        self.reset_view();
        let _ = self.tx_worker.send(WorkerRequest::Open { data, name });
        ctx.request_repaint();
    }

    pub fn reset_view(&mut self) {
        self.view.zoom = 1.0;
        self.view.pan = egui::Vec2::ZERO;
    }

    fn process_worker_messages(&mut self, ctx: &egui::Context) {
        // RR-15 Limit: GUI - Handle asynchronous background messages
        while let Ok(msg) = self.rx_worker.try_recv() {
            match msg {
                WorkerResponse::LoadingProgress { message } => {
                    self.loading_message = message;
                    ctx.request_repaint();
                }
                WorkerResponse::DocumentLoaded {
                    name,
                    num_pages,
                    page_sizes,
                    ust_root,
                    file_size,
                    version,
                    metadata,
                    security_method,
                    permissions,
                    fonts,
                } => {
                    self.pdf_name = name;
                    self.total_pages = num_pages;
                    self.compute_layouts(&page_sizes);
                    self.doc_page_sizes = page_sizes;

                    self.doc_file_size = Some(file_size);
                    self.doc_version = Some(version);
                    self.doc_metadata = Some(metadata);
                    self.doc_security_method = Some(security_method);
                    self.doc_permissions = permissions;
                    self.doc_fonts = fonts;

                    // Load parsed accessibility tag tree
                    self.ust_registry.root = ust_root;

                    // Kick off Matterhorn compliance audit asynchronously in the background
                    let _ = self.tx_worker.send(WorkerRequest::Audit);

                    self.is_loading = false;
                    ctx.request_repaint();
                }
                WorkerResponse::PageRendered { index, scene, text, spans, .. } => {
                    self.scenes.insert(index, scene);
                    self.request_queue.remove(&index);
                    self.invalidated_thumbnails.insert(index);

                    if let Some(text) = text {
                        self.raw_texts.insert(index, text);
                    }

                    if let Some(spans) = spans {
                        self.page_spans.insert(index, spans);
                    } else if let Some(text) = self.raw_texts.get(&index) {
                        if let Some(layout) = self.page_layouts.get(index) {
                            let size = layout.rect.size();
                            let spans =
                                SelectionManager::generate_spans_for_page(text, size.x, size.y);
                            self.page_spans.insert(index, spans);
                        }
                    }

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
                    ctx.request_repaint();
                }
                WorkerResponse::Error(err) => {
                    self.is_loading = false;
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
                    ui.colored_label(egui::Color32::RED, "GPU Compute Support Error");
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

        if response.drag_started()
            && let Some(pos) = screen_pos
        {
            let pdf_pos = SelectionManager::screen_to_pdf(page_screen_rect, zoom, unscaled_h, pos);
            self.signature_position =
                Some((visible_index, egui::Rect::from_min_max(pdf_pos, pdf_pos)));
        }

        if response.dragged()
            && let Some(pos) = screen_pos
            && let Some((sig_idx, sig_rect)) = &mut self.signature_position
        {
            if *sig_idx == visible_index {
                let pdf_pos =
                    SelectionManager::screen_to_pdf(page_screen_rect, zoom, unscaled_h, pos);
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

    fn handle_page_interactions(
        &mut self,
        ui: &mut egui::Ui,
        viewport_rect: egui::Rect,
        zoom: f32,
    ) {
        // RR-15 Limit: GUI - Unified egui pointer and canvas coordinate interaction loop
        let visible_pages = self.view.visible_pages.clone();
        for &visible_index in &visible_pages {
            if let Some(layout) = self.page_layouts.get(visible_index) {
                let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0)
                    + self.view.pan;
                let page_screen_rect = egui::Rect::from_min_size(
                    origin + layout.rect.min.to_vec2() * zoom,
                    layout.rect.size() * zoom,
                );
                let unscaled_h = layout.rect.height();

                if self.is_placing_signature {
                    self.handle_signature_placement_interaction(
                        ui,
                        visible_index,
                        page_screen_rect,
                        unscaled_h,
                        zoom,
                    );
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

    fn get_structural_highlight(
        &self,
        viewport_rect: egui::Rect,
        zoom: f32,
    ) -> Option<(usize, egui::Rect)> {
        let selected_id = self.ust_registry.selected_node_id?;
        if let Some(ref root) = self.ust_registry.root {
            if root.id == selected_id {
                return None;
            }
        }
        let rect = self.ust_registry.find_rect_by_id(selected_id)?;
        let page_idx = 0; // Default to first page
        let layout = self.page_layouts.get(page_idx)?;
        let origin =
            egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;
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

    fn get_signature_highlight(
        &self,
        viewport_rect: egui::Rect,
        zoom: f32,
    ) -> Option<(usize, egui::Rect)> {
        let (sig_page, sig_rect) = self.signature_position?;
        let layout = self.page_layouts.get(sig_page)?;
        let origin =
            egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;
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
                let origin = egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0)
                    + self.view.pan;
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
            &self.ust_registry,
            self.show_reading_order,
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
        let origin =
            egui::pos2(viewport_rect.center().x, viewport_rect.min.y + 20.0) + self.view.pan;

        for layout in &self.page_layouts {
            let page_screen_rect = egui::Rect::from_min_size(
                origin + layout.rect.min.to_vec2() * zoom,
                layout.rect.size() * zoom,
            );

            if viewport_rect.intersects(page_screen_rect) {
                if let Some(scene) = self.scenes.get(&layout.index) {
                    let unscaled_size = egui::vec2(layout.rect.width(), layout.rect.height());
                    visible_pages_data.push((
                        layout.index,
                        Arc::clone(scene),
                        page_screen_rect,
                        unscaled_size,
                    ));
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
        // RR-15 Limit: GUI - Update vello state and trigger document rendering
        let ctx = ui.ctx().clone();
        self.process_worker_messages(&ctx);

        if !self.check_gpu_support(ui, frame) {
            return;
        }

        let rs = match frame.wgpu_render_state() {
            Some(state) => state,
            None => return,
        };

        if self.clear_thumbnails_pending {
            if let Some(ref mut r) = self.vello_renderer {
                r.clear_thumbnails(rs);
            }
            self.clear_thumbnails_pending = false;
        }

        if !self.invalidated_thumbnails.is_empty() {
            if let Some(ref mut r) = self.vello_renderer {
                for page_idx in std::mem::take(&mut self.invalidated_thumbnails) {
                    r.invalidate_thumbnail(rs, page_idx);
                }
            }
        }

        self.queue_visible_pages();

        egui::CentralPanel::default().frame(egui::Frame::NONE).show_inside(ui, |ui| {
            let bg_color = egui::Color32::from_rgb(235, 237, 240);
            ui.painter().rect_filled(ui.max_rect(), 0.0, bg_color);

            if let Some(err) = &self.error {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::RED, err);
                });
            } else if !self.page_layouts.is_empty() {
                let viewport_rect = ui.max_rect();
                self.last_viewport_rect = Some(viewport_rect);
                self.render_document_panel(ui, rs, viewport_rect);

                // Floating page & zoom overlay at the top-center of the CentralPanel
                let overlay_width = 240.0;
                let overlay_height = 36.0;
                let overlay_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        viewport_rect.center().x - overlay_width / 2.0,
                        viewport_rect.top() + 16.0,
                    ),
                    egui::vec2(overlay_width, overlay_height),
                );

                // Rounded semi-transparent background card
                ui.painter().rect_filled(
                    overlay_rect,
                    6.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );
                ui.painter().rect_stroke(
                    overlay_rect,
                    6.0,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(200)),
                    egui::StrokeKind::Outside,
                );

                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(overlay_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                child_ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let current_page = self.view.visible_pages.first().cloned().unwrap_or(0);
                    if ui.button("◀").clicked() && current_page > 0 {
                        self.view.scroll_to_page(current_page - 1, &self.page_layouts);
                    }
                    ui.label(format!(" {} / {} ", current_page + 1, self.total_pages));
                    if ui.button("▶").clicked() && current_page + 1 < self.total_pages {
                        self.view.scroll_to_page(current_page + 1, &self.page_layouts);
                    }

                    ui.separator();
                    ui.label(format!("{:.0}%", self.view.zoom * 100.0));
                    if ui.button("Reset").clicked() {
                        self.reset_view();
                    }
                });
            } else if self.is_loading {
                ui.centered_and_justified(|ui| {
                    ui.label(&self.loading_message);
                });
            } else {
                // Keep the central panel blank at startup as requested
            }
        });
    }

    fn show_export_wizard_window(&mut self, ctx: &egui::Context) {
        crate::export_wizard::ExportWizard::show(self, ctx);
    }
}

impl FerruginousApp {
    fn apply_global_styles(&self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::light());
        ctx.global_style_mut(|style| {
            style.visuals.selection.stroke = egui::Stroke::NONE;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_gray(210));
        });
    }

    fn render_left_side_panels(&mut self, ui: &mut egui::Ui) {
        // 1. Left Icon Bar (Vertical column, full height)
        let locale_mgr = &self.locale_mgr;
        let active_lang = &self.active_language;

        egui::Panel::left("left_icon_bar").resizable(false).default_size(50.0).show_inside(
            ui,
            |ui| {
                self.sidebar_panel.show_icon_bar(ui, locale_mgr, active_lang);
            },
        );

        // 2. Context Panel (resizable, automatic size adjusting)
        if self.sidebar_panel.context_panel_open {
            egui::Panel::left("context_panel")
                .resizable(true)
                .show_separator_line(true)
                .size_range(260.0..=900.0)
                .show_inside(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    egui::Frame::NONE.inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                        self.sidebar_panel.show(
                            ui,
                            &mut self.ust_registry,
                            &self.tx_worker,
                            &self.pdf_name,
                            self.total_pages,
                            &self.doc_metadata,
                            self.doc_file_size,
                            &self.doc_version,
                            &self.doc_security_method,
                            self.doc_permissions,
                            &self.doc_page_sizes,
                            &self.doc_fonts,
                            locale_mgr,
                            active_lang,
                        );
                    });
                });
        }

        // 3. Arlington Dictionary Inspector (Left side, next to context panel)
        if self.show_inspector {
            let selected_tag = self.ust_registry.selected_node_id.and_then(|id| {
                if let Some(ref root) = self.ust_registry.root {
                    crate::sidebar::USTRegistry::find_node_by_id_recursive(root, id)
                        .map(|n| n.tag.as_str())
                } else {
                    None
                }
            });
            egui::Panel::left("inspector_panel")
                .resizable(true)
                .show_separator_line(true)
                .default_size(280.0)
                .size_range(200.0..=450.0)
                .show_inside(ui, |ui| {
                    egui::Frame::NONE.inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                        self.arlington_inspector.show(ui, selected_tag, locale_mgr, active_lang);
                    });
                });
        }
    }

    fn render_right_side_panels(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 1. Icon Bar (Right-most, 50px width)
        egui::Panel::right("icon_bar").resizable(false).default_size(50.0).show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 8.0);
                ui.add_space(8.0);
                let load_btn = egui::Button::new(egui::RichText::new("\u{e247}").size(16.0))
                    .min_size(egui::vec2(36.0, 36.0));
                if ui
                    .add(load_btn)
                    .on_hover_text(self.locale_mgr.tr(&self.active_language, "tooltip_load_pdf"))
                    .clicked()
                    && let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file()
                {
                    self.open_file(p, &ctx);
                }

                // Disable editing tools if no document loaded
                let has_doc = self.total_pages > 0;
                ui.add_enabled_ui(has_doc, |ui| {
                    ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 8.0);

                    // Redact Brush
                    let redact_is_active = self.redaction_manager.is_active;
                    let mut redact_btn =
                        egui::Button::new(egui::RichText::new("\u{e28f}").size(16.0))
                            .min_size(egui::vec2(36.0, 36.0));
                    if redact_is_active {
                        redact_btn =
                            redact_btn.stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(80)));
                    }
                    if ui
                        .add(redact_btn)
                        .on_hover_text(
                            self.locale_mgr.tr(&self.active_language, "tooltip_redact_brush"),
                        )
                        .clicked()
                    {
                        self.redaction_manager.is_active = !redact_is_active;
                        if self.redaction_manager.is_active {
                            self.selection_manager.clear();
                            self.selection_manager.is_tagging_brush_active = false;
                            self.caliper_tool.is_active = false;
                        }
                    }

                    // Tagging Brush
                    let tagging_is_active = self.selection_manager.is_tagging_brush_active;
                    let mut tagging_btn =
                        egui::Button::new(egui::RichText::new("\u{e17f}").size(16.0))
                            .min_size(egui::vec2(36.0, 36.0));
                    if tagging_is_active {
                        tagging_btn = tagging_btn
                            .stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(80)));
                    }
                    if ui
                        .add(tagging_btn)
                        .on_hover_text(
                            self.locale_mgr.tr(&self.active_language, "tooltip_tagging_brush"),
                        )
                        .clicked()
                    {
                        self.selection_manager.is_tagging_brush_active = !tagging_is_active;
                        if self.selection_manager.is_tagging_brush_active {
                            self.selection_manager.clear();
                            self.redaction_manager.is_active = false;
                            self.caliper_tool.is_active = false;
                        }
                    }

                    // Caliper Brush
                    let caliper_is_active = self.caliper_tool.is_active;
                    let mut caliper_btn =
                        egui::Button::new(egui::RichText::new("\u{e14b}").size(16.0))
                            .min_size(egui::vec2(36.0, 36.0));
                    if caliper_is_active {
                        caliper_btn = caliper_btn
                            .stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(80)));
                    }
                    if ui
                        .add(caliper_btn)
                        .on_hover_text(
                            self.locale_mgr.tr(&self.active_language, "tooltip_caliper_brush"),
                        )
                        .clicked()
                    {
                        self.caliper_tool.is_active = !caliper_is_active;
                        if self.caliper_tool.is_active {
                            self.selection_manager.clear();
                            self.redaction_manager.is_active = false;
                            self.selection_manager.is_tagging_brush_active = false;
                        }
                    }

                    // Inspector
                    let mut inspector_btn =
                        egui::Button::new(egui::RichText::new("\u{e151}").size(16.0))
                            .min_size(egui::vec2(36.0, 36.0));
                    if self.show_inspector {
                        inspector_btn = inspector_btn
                            .stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(80)));
                    }
                    if ui
                        .add(inspector_btn)
                        .on_hover_text(
                            self.locale_mgr.tr(&self.active_language, "tooltip_inspector"),
                        )
                        .clicked()
                    {
                        self.show_inspector = !self.show_inspector;
                    }

                    // Export PDF
                    let export_btn = egui::Button::new(egui::RichText::new("\u{e14d}").size(16.0))
                        .min_size(egui::vec2(36.0, 36.0));
                    if ui
                        .add(export_btn)
                        .on_hover_text(
                            self.locale_mgr.tr(&self.active_language, "tooltip_export_pdf"),
                        )
                        .clicked()
                    {
                        self.show_export_wizard = true;
                    }
                });

                let current_height = ui.available_height();
                if current_height > 100.0 {
                    ui.add_space(current_height - 90.0);
                }

                // Settings Button
                let settings_btn = egui::Button::new(egui::RichText::new("\u{e30b}").size(16.0))
                    .min_size(egui::vec2(36.0, 36.0));
                if ui
                    .add(settings_btn)
                    .on_hover_text(self.locale_mgr.tr(&self.active_language, "tooltip_settings"))
                    .clicked()
                {
                    self.show_settings_modal = true;
                }

                // About Button
                let about_btn = egui::Button::new(egui::RichText::new("\u{e082}").size(16.0))
                    .min_size(egui::vec2(36.0, 36.0));
                if ui
                    .add(about_btn)
                    .on_hover_text(self.locale_mgr.tr(&self.active_language, "tooltip_about"))
                    .clicked()
                {
                    self.show_about_modal = true;
                }
            });
        });

        // 2. Thumbnails Panel (200px width, inner-right)
        crate::thumbnail_sidebar::ThumbnailSidebar::show(self, ui, frame);
    }

    fn render_status_bar(&self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status_bar").default_size(28.0).resizable(false).show_inside(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.locale_mgr.tr(&self.active_language, "status_ready"));
                    ui.separator();
                    if self.total_pages > 0 {
                        let current_page = self.view.visible_pages.first().cloned().unwrap_or(0);
                        let indicator = self
                            .locale_mgr
                            .tr(&self.active_language, "page_indicator")
                            .replacen("{}", &(current_page + 1).to_string(), 1)
                            .replacen("{}", &self.total_pages.to_string(), 1);
                        ui.label(indicator);
                    } else {
                        ui.label(self.locale_mgr.tr(&self.active_language, "no_doc_loaded"));
                    }
                    ui.separator();
                    if self.show_reading_order {
                        ui.label(
                            self.locale_mgr.tr(&self.active_language, "reading_order_enabled"),
                        );
                    } else {
                        ui.label(
                            self.locale_mgr.tr(&self.active_language, "reading_order_disabled"),
                        );
                    }
                });
            },
        );
    }

    fn render_overlay_windows(&mut self, ctx: &egui::Context) {
        if self.show_export_wizard {
            self.show_export_wizard_window(ctx);
        }

        // Show Command Palette window overlay
        crate::command_palette::CommandPalette::show(self, ctx);

        // Show interactive Create Semantic Tag popup dialog on visual tag selector brush highlights
        if let Some(req) = self.selection_manager.pending_tag_request.clone() {
            let mut show_popup = true;
            let popup_title = self.locale_mgr.tr(&self.active_language, "tag_popup_title");
            egui::Window::new(popup_title)
                .open(&mut show_popup)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(self.locale_mgr.tr(&self.active_language, "tag_popup_selected"));
                    ui.group(|ui| {
                        ui.label(&req.text);
                    });
                    ui.add_space(5.0);
                    ui.label(self.locale_mgr.tr(&self.active_language, "tag_popup_instruction"));

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

                    if ui
                        .button(self.locale_mgr.tr(&self.active_language, "tag_popup_cancel"))
                        .clicked()
                    {
                        self.selection_manager.pending_tag_request = None;
                    }
                });
            if !show_popup {
                self.selection_manager.pending_tag_request = None;
            }
        }

        // Show Settings Modal
        if self.show_settings_modal {
            let mut show_settings = true;
            let title = self.locale_mgr.tr(&self.active_language, "settings_title");
            egui::Window::new(title)
                .open(&mut show_settings)
                .resizable(false)
                .collapsible(false)
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                self.locale_mgr
                                    .tr(&self.active_language, "settings_language_label"),
                            );
                            let current_lang = self.active_language.clone();
                            egui::ComboBox::from_id_salt("settings_lang_combobox")
                                .selected_text(&current_lang)
                                .show_ui(ui, |ui| {
                                    for lang in self.locale_mgr.available_languages() {
                                        ui.selectable_value(
                                            &mut self.active_language,
                                            lang.clone(),
                                            lang,
                                        );
                                    }
                                });
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.vertical_centered(|ui| {
                            if ui
                                .button(self.locale_mgr.tr(&self.active_language, "settings_close"))
                                .clicked()
                            {
                                self.show_settings_modal = false;
                            }
                        });
                    });
                });
            if !show_settings {
                self.show_settings_modal = false;
            }
        }

        // Show About Modal
        if self.show_about_modal {
            let mut show_about = true;
            let about_title = self.locale_mgr.tr(&self.active_language, "about_title");
            egui::Window::new(about_title)
                .open(&mut show_about)
                .resizable(false)
                .collapsible(false)
                .default_width(320.0)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(
                                self.locale_mgr.tr(&self.active_language, "about_app_name"),
                            )
                            .strong()
                            .size(18.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} 0.1.0",
                                self.locale_mgr.tr(&self.active_language, "about_version")
                            ))
                            .weak(),
                        );
                        ui.add_space(8.0);
                        ui.label(self.locale_mgr.tr(&self.active_language, "about_description"));
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                self.locale_mgr.tr(&self.active_language, "about_third_party"),
                            )
                            .strong(),
                        );
                        ui.add_space(4.0);
                    });

                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        let credits = [
                            ("lopdf", "MIT License", "Low-level PDF parsing"),
                            ("pdf-writer", "Apache-2.0 License", "PDF object serialization"),
                            ("vello", "Apache-2.0 / MIT", "GPU vector graphics"),
                            ("egui / eframe", "MIT / Apache-2.0", "GUI library"),
                            ("Lucide Icons", "ISC License", "Icon font asset"),
                        ];
                        for (name, license, purpose) in credits {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(name).strong());
                                ui.label(format!("({})", license));
                            });
                            ui.label(egui::RichText::new(purpose).weak());
                            ui.add_space(4.0);
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if ui
                            .button(self.locale_mgr.tr(&self.active_language, "about_close"))
                            .clicked()
                        {
                            self.show_about_modal = false;
                        }
                    });
                });
            if !show_about {
                self.show_about_modal = false;
            }
        }
    }
}

impl eframe::App for FerruginousApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // RR-15 Limit: Dispatcher - Main application UI shell layout routing layout panels and windows
        let ctx = ui.ctx().clone();
        self.apply_global_styles(&ctx);

        let entire_rect = ui.max_rect();
        ui.painter().rect_filled(entire_rect, 0.0, ui.visuals().window_fill);

        self.render_left_side_panels(ui);
        self.render_right_side_panels(ui, frame);
        self.render_status_bar(ui);

        self.update_vello(ui, frame);
        self.render_overlay_windows(&ctx);
    }
}
