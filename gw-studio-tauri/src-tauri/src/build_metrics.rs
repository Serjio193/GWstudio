use serde::{Deserialize, Serialize};
use std::fs;

use crate::thumbnails::thumbnail_cache_path;

#[derive(Deserialize)]
pub(crate) struct BuildMetricsRequest {
    emulator: String,
    titles: Vec<String>,
    rom_paths: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct BuildMetrics {
    rom_bytes: u64,
    image_bytes: u64,
    rom_count: usize,
    image_count: usize,
}

#[tauri::command]
pub(crate) fn compute_build_metrics(request: BuildMetricsRequest) -> Result<BuildMetrics, String> {
    let mut rom_bytes = 0_u64;
    let mut image_bytes = 0_u64;
    let mut image_count = 0_usize;
    let rom_count = request.rom_paths.len();

    for rom_path in &request.rom_paths {
        match fs::metadata(rom_path) {
            Ok(metadata) if metadata.is_file() => {
                rom_bytes = rom_bytes.saturating_add(metadata.len());
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    for title in request.titles {
        let path = thumbnail_cache_path(&request.emulator, &title);
        match fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => {
                image_bytes = image_bytes.saturating_add(metadata.len());
                image_count += 1;
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    Ok(BuildMetrics {
        rom_bytes,
        image_bytes,
        rom_count,
        image_count,
    })
}
