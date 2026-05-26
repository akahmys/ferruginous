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
    pdf_path: Option<PathBuf>,

    vello_renderer: Option<VelloRenderer>,
    scenes: BTreeMap<usize, Arc<Scene>>,
    request_queue: BTreeSet<usize>,
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
            pdf_path: None,
            vello_renderer,
            scenes: BTreeMap::new(),
            request_queue: BTreeSet::new(),
        }
    }

    pub fn open_file(&mut self, path: PathBuf, _ctx: &egui::Context) {
        self.error = None;
        self.total_pages = 0;
        self.page_layouts.clear();
        self.scenes.clear();
        self.request_queue.clear();
        self.reset_view();
        let _ = self.tx_worker.send(WorkerRequest::Open(path));
    }

    fn reset_view(&mut self) {
        self.view.zoom = 1.0;
        self.view.pan = egui::Vec2::ZERO;
    }

    fn process_worker_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx_worker.try_recv() {
            match msg {
                WorkerResponse::DocumentLoaded { path, num_pages, page_sizes } => {
                    self.pdf_path = Some(path);
                    self.total_pages = num_pages;
                    self.compute_layouts(&page_sizes);
                    ctx.request_repaint();
                }
                WorkerResponse::PageRendered { index, scene, .. } => {
                    self.scenes.insert(index, scene);
                    self.request_queue.remove(&index);
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

    fn update_vello(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.process_worker_messages(&ctx);

        let Some(vello_renderer) = &mut self.vello_renderer else { return };
        let Some(rs) = frame.wgpu_render_state() else { return };

        for &visible_index in &self.view.visible_pages {
            if !self.scenes.contains_key(&visible_index)
                && !self.request_queue.contains(&visible_index)
            {
                let scale = 2.0;
                self.request_queue.insert(visible_index);
                let _ =
                    self.tx_worker.send(WorkerRequest::RenderPage { index: visible_index, scale });
            }
        }

        let mut active_textures = BTreeMap::new();
        for &visible_index in &self.view.visible_pages {
            if let (Some(scene), Some(layout)) =
                (self.scenes.get(&visible_index), self.page_layouts.get(visible_index))
            {
                let scale = 2.0;
                let width = (layout.rect.width() * scale).round() as u32;
                let height = (layout.rect.height() * scale).round() as u32;

                if let Some(tid) =
                    vello_renderer.render_page(rs, scene, visible_index, width, height)
                {
                    active_textures.insert(visible_index, tid);
                }
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(err) = &self.error {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::RED, err);
                });
            } else if !self.page_layouts.is_empty() {
                self.view.show(ui, &self.page_layouts, &active_textures);
            } else if self.pdf_path.is_some() {
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

    fn show_top_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("top_bar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open PDF").clicked()
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
                    if let Some(p) = &self.pdf_path {
                        ui.separator();
                        ui.label(p.file_name().unwrap_or_default().to_string_lossy());
                    }
                }
            });
        });
    }
}

impl eframe::App for FerruginousApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.show_top_bar(ui);
        self.update_vello(ui, frame);
    }
}
