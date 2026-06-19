use crate::paths::{host_root, workspace_root};
use base64::Engine;
use image::ImageFormat;
use serde::Deserialize;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Deserialize)]
pub(crate) struct ThumbnailCacheRequest {
    emulator: String,
    title: String,
}

#[derive(Deserialize)]
pub(crate) struct ThumbnailSaveRequest {
    emulator: String,
    title: String,
    bytes: Vec<u8>,
}

pub(crate) fn thumbnails_dir() -> PathBuf {
    host_root().join("content").join("image")
}

fn legacy_thumbnails_dir() -> PathBuf {
    workspace_root().join("thumbnails")
}

pub(crate) fn sanitize_thumbnail_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_' | '(' | ')' | ',' | '.' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub(crate) fn thumbnail_cache_path(emulator: &str, title: &str) -> PathBuf {
    thumbnails_dir()
        .join(sanitize_thumbnail_part(emulator))
        .join(format!("{}.png", sanitize_thumbnail_part(title)))
}

fn legacy_thumbnail_cache_path(emulator: &str, title: &str) -> PathBuf {
    legacy_thumbnails_dir()
        .join(sanitize_thumbnail_part(emulator))
        .join(format!("{}.png", sanitize_thumbnail_part(title)))
}

fn normalize_thumbnail_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let source =
        image::load_from_memory(bytes).map_err(|error| format!("failed to decode image: {error}"))?;
    let mut cursor = Cursor::new(Vec::new());
    source
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| format!("failed to encode thumbnail png: {error}"))?;
    Ok(cursor.into_inner())
}

#[tauri::command]
pub(crate) fn load_thumbnail_cache(request: ThumbnailCacheRequest) -> Result<Option<String>, String> {
    let path = thumbnail_cache_path(&request.emulator, &request.title);
    let source_path = if path.exists() {
        path
    } else {
        let legacy_path = legacy_thumbnail_cache_path(&request.emulator, &request.title);
        if !legacy_path.exists() {
            return Ok(None);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("failed to create thumbnail cache dir: {error}"))?;
        }
        let _ = fs::copy(&legacy_path, &path);
        if path.exists() { path } else { legacy_path }
    };
    if !source_path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&source_path).map_err(|error| format!("failed to read thumbnail cache: {error}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:image/png;base64,{encoded}")))
}

#[tauri::command]
pub(crate) fn save_thumbnail_cache(request: ThumbnailSaveRequest) -> Result<String, String> {
    let path = thumbnail_cache_path(&request.emulator, &request.title);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("failed to create thumbnail cache dir: {error}"))?;
    }
    let normalized_png = normalize_thumbnail_png(&request.bytes)?;
    fs::write(&path, normalized_png).map_err(|error| format!("failed to write thumbnail cache: {error}"))?;
    Ok(path.to_string_lossy().to_string())
}
