//! System services abstraction layer for platform independence.
//! 
//! This module defines traits for services that depend on the operating system
//! or windowing environment, such as file dialogs, thermal management, or
//! native notifications.

use std::path::PathBuf;

/// Interface for platform-specific system services.
pub trait SystemBridge: Send + Sync {
    /// Opens a native file dialog to pick a single file.
    fn pick_file(&self, title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf>;
    
    /// Returns the name of the current platform.
    fn platform_name(&self) -> String;
}

/// Implementation of `SystemBridge` using the `rfd` crate for native dialogs.
pub struct NativeBridge;

impl SystemBridge for NativeBridge {
    fn pick_file(&self, title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }
        dialog.pick_file()
    }

    fn platform_name(&self) -> String {
        if cfg!(target_os = "macos") {
            "macOS".to_string()
        } else if cfg!(target_os = "windows") {
            "Windows".to_string()
        } else if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else {
            "Unknown".to_string()
        }
    }
}
