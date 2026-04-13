//! MacOS-specific hardware stabilization and font loading.

use eframe::{egui, egui_wgpu};
use egui::{FontDefinitions, FontData, FontFamily};
use std::sync::Arc;

/// Returns WGPU native options optimized for Intel Iris Plus on macOS.
pub fn get_native_options() -> eframe::NativeOptions {
    let mut native_options = eframe::NativeOptions::default();
    native_options.renderer = eframe::Renderer::Wgpu;
    
    // INTEL GPU STABILIZATION
    native_options.wgpu_options.present_mode = wgpu::PresentMode::AutoVsync;
    native_options.wgpu_options.wgpu_setup = egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew {
        instance_descriptor: wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..Default::default()
        },
        power_preference: wgpu::PowerPreference::LowPower,
        device_descriptor: Arc::new(|_adapter| wgpu::DeviceDescriptor {
            label: Some("egui_device_macos"),
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            ..Default::default()
        }),
        ..Default::default()
    });

    native_options
}

/// Loads the Hiragino Sans system font for reliable Japanese rendering.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    
    // MacOS system font fallback for CJK
    if let Ok(font_data) = std::fs::read("/System/Library/Fonts/Hiragino Sans GB.ttc") {
        fonts.font_data.insert(
            "hiragino".to_owned(),
            Arc::new(FontData::from_owned(font_data))
        );
        
        fonts.families.get_mut(&FontFamily::Proportional).unwrap_or(&mut Vec::new())
            .insert(0, "hiragino".to_owned());
        fonts.families.get_mut(&FontFamily::Monospace).unwrap_or(&mut Vec::new())
            .push("hiragino".to_owned());
    }
    
    ctx.set_fonts(fonts);
}
