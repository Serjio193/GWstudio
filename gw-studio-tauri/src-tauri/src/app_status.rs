use serde::Serialize;

use crate::device::gnwmanager_argv;
use crate::paths::{host_root, runtime_tools_dir, workspace_root};
use crate::thumbnails::thumbnails_dir;

#[derive(Serialize)]
pub(crate) struct RuntimeStatus {
    workspace_root: String,
    logs_dir: String,
    tools_dir: String,
    thumbnails_dir: String,
    host_root: String,
    gnwmanager_source: String,
    rust_backend: &'static str,
}

#[tauri::command]
pub(crate) fn runtime_status() -> RuntimeStatus {
    RuntimeStatus {
        workspace_root: workspace_root().to_string_lossy().to_string(),
        logs_dir: workspace_root().join("logs").to_string_lossy().to_string(),
        tools_dir: runtime_tools_dir().to_string_lossy().to_string(),
        thumbnails_dir: thumbnails_dir().to_string_lossy().to_string(),
        host_root: host_root().to_string_lossy().to_string(),
        gnwmanager_source: if gnwmanager_argv().len() == 1 {
            "bundled".to_string()
        } else {
            "python-module".to_string()
        },
        rust_backend: "active",
    }
}

#[tauri::command]
pub(crate) fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
