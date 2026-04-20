use crate::view::PDFView;
use bytes::Bytes;
use ferruginous_render::{RenderBackend, VelloBackend};
use ferruginous_sdk::PdfDocument;
use pollster::block_on;
use std::path::PathBuf;

pub struct FerruginousApp {
    doc: Option<PdfDocument>,
    current_page: usize,
    total_pages: usize,
    view: PDFView,
    error: Option<String>,
    pdf_path: Option<PathBuf>,
}

impl FerruginousApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            doc: None,
            current_page: 0,
            total_pages: 0,
            view: PDFView::new(),
            error: None,
            pdf_path: None,
        }
    }

    pub fn open_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        let data = match std::fs::read(&path) {
            Ok(d) => Bytes::from(d),
            Err(e) => {
                self.error = Some(format!("Failed to read file: {}", e));
                return;
            }
        };

        match PdfDocument::open(data) {
            Ok(doc) => {
                self.total_pages = doc.page_count().unwrap_or(0);
                self.doc = Some(doc);
                self.current_page = 0;
                self.pdf_path = Some(path);
                self.error = None;
                self.reset_view();
                self.render_current_page(ctx);
            }
            Err(e) => {
                self.error = Some(format!("Failed to load PDF: {}", e));
            }
        }
    }

    fn reset_view(&mut self) {
        self.view.zoom = 1.0;
        self.view.pan = egui::Vec2::ZERO;
    }

    fn render_current_page(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.doc else { return };

        let (p_w, p_h) = doc.get_page_size(self.current_page).unwrap_or((595.0, 842.0));
        let width = (p_w * 2.0).round() as u32;
        let height = (p_h * 2.0).round() as u32;

        let mut backend = VelloBackend::new();
        // Match the 2x scaling used for the texture resolution
        backend.transform(kurbo::Affine::scale(2.0));

        if let Ok(()) = doc.render_page(self.current_page, &mut backend) {
            #[allow(dead_code)]
            let scene = backend.scene().clone();

            match block_on(ferruginous_render::headless::render_to_bytes(&scene, width, height)) {
                Ok(bytes) => {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [width as usize, height as usize],
                        &bytes,
                    );
                    let texture = ctx.load_texture(
                        format!("pdf_page_{}", self.current_page),
                        color_image,
                        Default::default(),
                    );
                    self.view.texture = Some(texture);
                }
                Err(e) => {
                    self.error = Some(format!("Rendering Error: {}", e));
                }
            }
        }
    }

    fn next_page(&mut self, ctx: &egui::Context) {
        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
            self.render_current_page(ctx);
        }
    }

    fn prev_page(&mut self, ctx: &egui::Context) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.render_current_page(ctx);
        }
    }
}

impl eframe::App for FerruginousApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open PDF").clicked()
                    && let Some(path) =
                        rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file()
                {
                    self.open_file(path, ctx);
                }

                ui.separator();

                if self.doc.is_some() {
                    if ui.button("<<").clicked() {
                        self.prev_page(ctx);
                    }
                    ui.label(format!("Page {} / {}", self.current_page + 1, self.total_pages));
                    if ui.button(">>").clicked() {
                        self.next_page(ctx);
                    }

                    if ui.button("Reset View").clicked() {
                        self.reset_view();
                    }

                    ui.separator();
                    ui.label(format!("Zoom: {:.1}%", self.view.zoom * 100.0));

                    if let Some(path) = &self.pdf_path {
                        ui.separator();
                        ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::RED, err);
                });
            } else if self.doc.is_some() {
                self.view.show(ui);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("Ferruginous");
                    ui.label("Fast, Secure, GPU-Accelerated PDF Viewer");
                    ui.add_space(20.0);
                    if ui.button("Open a PDF").clicked()
                        && let Some(path) =
                            rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file()
                    {
                        self.open_file(path, ctx);
                    }
                });
            }
        });
    }
}
