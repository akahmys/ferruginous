//! Ferruginous Native PDF Viewer Application.
//! (ISO 32000-2:2020 compliant core with Vello/WGPU rendering)

use eframe::egui;
use egui::{Visuals, Color32, Vec2, Key};
use ferruginous_render::{RenderBackend, VelloBackend, BackendOptions};
use ferruginous_sdk::core::Resolver;
use ferruginous_sdk::graphics::Affine;

mod types;
mod widgets;
mod sys;
use crate::sys::bridge::{SystemBridge, NativeBridge};

use crate::types::ToolMode;
/// The factor by which to super-sample the PDF for high-DPI quality.
pub const SUPER_SAMPLE_FACTOR: f32 = 1.5;
/// Atomic counter used to track and trigger rendering updates in the UI.
pub static RENDER_TRIGGER_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The primary application state for the Ferruginous UI.
#[allow(clippy::struct_excessive_bools)]
pub struct FerruginousApp {
    /// Count of frames rendered since startup.
    pub frame_count: u32,
    /// Currently active navigation tab (e.g. "Page", "Outline").
    pub active_tab: String,
    /// Currently selected interaction tool.
    pub tool_mode: ToolMode,
    /// Flag indicating if Japanese fonts are loaded.
    pub font_loaded: bool,
    /// Flag indicating if symbol/icon fonts are loaded.
    pub icons_loaded: bool,
    
    // PDF Document State
    /// The currently loaded PDF document, if any.
    pub pdf_doc: Option<ferruginous_sdk::loader::PdfDocument>,
    /// Index of the currently displayed page (0-indexed).
    pub current_page: usize,
    /// Total number of pages in the loaded document.
    pub page_count: usize,
    /// Current error message to be displayed in the UI.
    pub error_message: Option<String>,
    
    // Rendering Resources
    /// High-level PDF renderer backend.
    pub pdf_renderer: Box<dyn RenderBackend>,
    /// Arc reference to the wgpu device.
    pub device: Option<std::sync::Arc<wgpu::Device>>,
    /// reference to the wgpu queue.
    pub queue: Option<wgpu::Queue>,
    
    // Viewport State
    /// Current zoom level (1.0 = 100%).
    pub zoom_factor: f32,
    /// Current scroll/pan offset from center.
    pub pan_offset: egui::Vec2,

    // Search State
    /// The current search query string.
    pub search_query: String,
    /// List of search results found in the current document (or page).
    pub search_results: Vec<ferruginous_sdk::search::SearchResult>,
    /// Optional Content Group (Layer) visibility context.
    pub oc_context: Option<ferruginous_sdk::ocg::OCContext>,
    /// List of available OCG layers in the current document.
    pub available_ocgs: Vec<ferruginous_sdk::ocg::OptionalContentGroup>,
    
    /// Nominal size of the current PDF page in points.
    pub current_page_size: egui::Vec2,
    /// Size of the current page texture in pixels.
    pub page_texture_size: (u32, u32),

    /// trace: Number of draw operations in the last rendered list.
    pub last_draw_op_count: usize,
    /// Error message from Vello initialization, if any.
    pub vello_init_error: Option<String>,
    /// GPU adapter name.
    pub gpu_name: String,
    /// Persistence: Cached WGPU render state for manual texture registration.
    pub cached_render_state: Option<egui_wgpu::RenderState>,
    /// Intermediate texture for Vello rendering.
    pub vello_texture_view: Option<wgpu::TextureView>,
    /// Persistent ID for the registered Vello texture in egui.
    pub vello_texture_id: Option<egui::TextureId>,
    /// Counter for verifying callback execution.
    pub vello_callback_count: u32,

    // Diagnostic State
    /// Whether to show the debug overlay.
    pub show_debug_overlay: bool,
    /// Whether to enable diagnostic mode (detailed logging, etc.).
    pub diagnostic_mode: bool,
    
    // Platform Bridge
    /// Bridge to OS-specific features (file dialogs, etc.).
    pub system: Box<dyn SystemBridge>,
}

impl FerruginousApp {
    /// Creates a new instance of the application.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut device: Option<std::sync::Arc<wgpu::Device>> = None;
        let mut queue: Option<wgpu::Queue> = None;
        let mut gpu_name = "Unknown".to_string();
        let mut vello_init_error = None;

        sys::setup_fonts(&cc.egui_ctx);

        if let Some(render_state) = &cc.wgpu_render_state {
            device = Some(render_state.device.clone().into());
            queue = Some(render_state.queue.clone());
            gpu_name = render_state.adapter.get_info().name;
        } else {
            vello_init_error = Some("WGPU Render State missing".to_string());
        }

        Self {
            frame_count: 0,
            active_tab: "Page".to_string(),
            tool_mode: ToolMode::Select,
            font_loaded: false,
            icons_loaded: false,
            pdf_doc: None,
            current_page: 0,
            page_count: 0,
            error_message: None,
            pdf_renderer: Box::new(VelloBackend::new()),
            device,
            queue,
            zoom_factor: 1.0,
            pan_offset: egui::Vec2::ZERO,
            search_query: String::new(),
            search_results: Vec::new(),
            oc_context: None,
            available_ocgs: Vec::new(),
            current_page_size: egui::vec2(800.0, 1100.0), // Default until load
            page_texture_size: (0, 0),
            last_draw_op_count: 0,
            vello_init_error,
            gpu_name,
            cached_render_state: cc.wgpu_render_state.clone(),
            vello_texture_view: None,
            vello_texture_id: None,
            vello_callback_count: 0,
            show_debug_overlay: true,
            diagnostic_mode: false,
            system: Box::new(NativeBridge),
        }
    }

    /// Loads an initial PDF file from a path (CLI support).
    pub fn load_initial_file(&mut self, path: String) {
        if let Ok(data) = std::fs::read(&path) {
            match ferruginous_sdk::loader::load_document_structure(&data) {
                Ok(doc) => { 
                    eprintln!("[TRACE][UI] Loading initial file from CLI: {path}");
                    self.process_loaded_doc(doc); 
                }
                Err(e) => { 
                    eprintln!("[ERROR][UI] Failed to load initial file: {e:?}");
                    self.error_message = Some(format!("PDF Load Failed: {e:?}")); 
                }
            }
        }
    }

    /// Opens a file dialog to select and load a PDF document.
    pub fn open_pdf(&mut self) {
        self.error_message = None;
        if let Some(path) = self.system.pick_file("Open PDF", &[("PDF", &["pdf"])]) {
            if let Ok(data) = std::fs::read(&path) {
                match ferruginous_sdk::loader::load_document_structure(&data) {
                    Ok(doc) => { self.process_loaded_doc(doc); }
                    Err(e) => { self.error_message = Some(format!("PDF Load Failed: {e:?}")); }
                }
            }
        }
    }

    fn process_loaded_doc(&mut self, doc: ferruginous_sdk::loader::PdfDocument) {
        let tree = match doc.page_tree() {
            Ok(t) => t,
            Err(e) => {
                self.error_message = Some(format!("Page Tree Analysis Error: {e:?}"));
                return;
            }
        };
        self.page_count = tree.get_count();
        self.pdf_doc = Some(doc.clone());
        self.current_page = 0;
        self.pan_offset = Vec2::ZERO;

        // Extract OCG Layers if available
        if let Ok(catalog) = doc.catalog() {
            if let Some(ocp) = catalog.oc_properties() {
                self.oc_context = Some(ferruginous_sdk::ocg::OCContext::new(ocp.default_state()));
                self.available_ocgs.clear();
                for &r in &ocp.ocgs {
                    if let Ok(ferruginous_sdk::core::Object::Dictionary(d)) = doc.resolver().resolve(&r) {
                        if let Some(ferruginous_sdk::core::Object::String(n)) = d.get(b"Name".as_ref()) {
                            self.available_ocgs.push(ferruginous_sdk::ocg::OptionalContentGroup {
                                reference: r, name: n.to_vec(), intent: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
        
        // Update current page size
        if let Ok(tree) = doc.page_tree() {
            if let Ok(page) = tree.get_page(self.current_page) {
                if let Some(bbox) = page.media_box_array() {
                    self.current_page_size = egui::vec2((bbox[2] - bbox[0]).abs() as f32, (bbox[3] - bbox[1]).abs() as f32);
                }
            }
        }

        self.update_rendering();
    }

    /// Updates the Vello scene based on the current document and page state.
    pub fn update_rendering(&mut self) {
        // Sync page size from document
        if let Some(doc) = &self.pdf_doc {
            if let Ok(tree) = doc.page_tree() {
                if let Ok(page) = tree.get_page(self.current_page) {
                    if let Some(bbox) = page.media_box_array() {
                        self.current_page_size = egui::vec2((bbox[2] - bbox[0]).abs() as f32, (bbox[3] - bbox[1]).abs() as f32);
                        eprintln!("[DEBUG] Updated page size: {:?}", self.current_page_size);
                    }
                }
            }
        }

        // PDF points (1/72 inch) to pixels. 1.0 zoom = 1.0 points.
        // We add a slight scale boost to standard resolution for high-DPI quality.
        // The SDK's DisplayList already includes a normalization transform (flip/shift) 
        // as its first command, so we only apply the root scaling here.
        let render_transform = Affine::scale(self.zoom_factor as f64 * SUPER_SAMPLE_FACTOR as f64);
        
        eprintln!("[DEBUG] Root Render Scale: {render_transform:?}");

        self.pdf_renderer.clear();
        
        // Ensure texture is initialized if we have a render state
        if self.vello_texture_view.is_none() {
            if let Some(state) = &self.cached_render_state {
                let size = wgpu::Extent3d { width: 2048, height: 2048, depth_or_array_layers: 1 };
                let texture = state.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Vello Target"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                self.vello_texture_view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));

                // Register the texture with egui renderer
                if let Some(view) = &self.vello_texture_view {
                    let mut egui_renderer = state.renderer.write();
                    self.vello_texture_id = Some(egui_renderer.register_native_texture(
                        &state.device,
                        view,
                        wgpu::FilterMode::Linear,
                    ));
                }
            }
        }

        if let Some(doc) = &self.pdf_doc {
            if let Ok(tree) = doc.page_tree() {
                if let Ok(page) = tree.get_page(self.current_page) {
                    match page.get_display_list() {
                        Ok(list) => {
                            self.last_draw_op_count = list.len();
                            eprintln!("[DEBUG] Display list updated, commands: {}", list.len());
                            self.pdf_renderer.render_display_list(
                                &list, 
                                render_transform, 
                                self.oc_context.as_ref()
                            );
                        }
                        Err(e) => {
                            eprintln!("[DEBUG] get_display_list() failed: {e:?}");
                        }
                    }
                }
            }
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = Visuals::light();
        let primary_rust = Color32::from_rgb(183, 65, 14);
        let bg_premium = Color32::from_rgb(249, 249, 251);
        let text_dark = Color32::from_rgb(28, 28, 30);
        let border_subtle = Color32::from_rgb(220, 220, 225);

        visuals.panel_fill = bg_premium;
        visuals.window_fill = Color32::WHITE;
        visuals.window_corner_radius = egui::CornerRadius::same(4);
        
        // Buttons
        visuals.widgets.active.bg_fill = primary_rust;
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);
        
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(235, 235, 240);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, primary_rust);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
        
        visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_dark);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
        
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, border_subtle);
        
        visuals.override_text_color = Some(text_dark);
        ctx.set_visuals(visuals);
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut trigger_update = false;

        ctx.input(|i| {
            // Page Navigation
            if i.key_pressed(Key::ArrowRight) && self.current_page + 1 < self.page_count {
                self.current_page += 1;
                self.pan_offset = Vec2::ZERO;
                trigger_update = true;
            }
            if i.key_pressed(Key::ArrowLeft) && self.current_page > 0 {
                self.current_page -= 1;
                self.pan_offset = Vec2::ZERO;
                trigger_update = true;
            }

            // Zoom
            if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) || i.modifiers.command && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)) {
                self.zoom_factor = (self.zoom_factor + 0.1).min(10.0);
                trigger_update = true;
            }
            if i.key_pressed(Key::Minus) || (i.modifiers.command && i.key_pressed(Key::Minus)) {
                self.zoom_factor = (self.zoom_factor - 0.1).max(0.1);
                trigger_update = true;
            }

            // View Resets
            if i.key_pressed(Key::Num0) && i.modifiers.command {
                self.zoom_factor = 1.0;
                self.pan_offset = Vec2::ZERO;
                trigger_update = true;
            }
        });

        if trigger_update {
            self.update_rendering();
        }
    }
}

impl eframe::App for FerruginousApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // --- 1. Direct Vello Rendering ---
        if let Some(render_state) = frame.wgpu_render_state() {
            let device = &render_state.device;
            let queue = &render_state.queue;
            
            if self.vello_texture_id.is_none() {
                if let Some(view) = &self.vello_texture_view {
                    let id = render_state.renderer.write().register_native_texture(
                        device,
                        view,
                        wgpu::FilterMode::Linear
                    );
                    self.vello_texture_id = Some(id);
                }
            }

                if let Some(target_view) = &self.vello_texture_view {
                    let width = (self.current_page_size.x * SUPER_SAMPLE_FACTOR) as u32;
                    let height = (self.current_page_size.y * SUPER_SAMPLE_FACTOR) as u32;

                    // Initialize renderer if needed
                    if self.frame_count == 0 || self.vello_callback_count == 0 {
                         let _ = self.pdf_renderer.prepare_renderer(device, BackendOptions { use_cpu: false, antialiasing: true });
                    }

                    match self.pdf_renderer.render_to_texture(
                        device,
                        queue,
                        target_view,
                        width,
                        height,
                    ) {
                        Ok(()) => {
                            RENDER_TRIGGER_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(e) => {
                            eprintln!("[ERROR] Rendering failed: {e}");
                        }
                    }
                }
        }
        self.vello_callback_count = RENDER_TRIGGER_COUNT.load(std::sync::atomic::Ordering::SeqCst);

        self.apply_theme(ctx);
        self.handle_shortcuts(ctx);
        
        widgets::header::show_header(self, ctx);
        widgets::sidebar::show_sidebar(self, ctx);
        widgets::canvas::show_canvas(self, ctx);
        widgets::debug::show_debug_overlay(self, ctx);

        if self.frame_count == 1 && self.pdf_doc.is_some() {
            eprintln!("[TRACE][UI] Initial render complete for document");
        }

        self.frame_count += 1;
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let mut native_options = sys::get_native_options();
    
    native_options.viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 850.0])
        .with_title("Ferruginous PDF Tool")
        .with_active(true);
        
    eframe::run_native(
        "ferruginous-pdf-ui",
        native_options,
        Box::new(|cc| {
            let mut app = FerruginousApp::new(cc);
            if let Some(arg) = std::env::args().nth(1) {
                app.load_initial_file(arg);
            }
            Ok(Box::new(app))
        }),
    )
}
