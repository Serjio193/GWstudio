use image::imageops::FilterType;
use image::ImageFormat;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::paths::host_root;

fn sanitize_file_part(value: &str) -> String {
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

pub(crate) fn emulator_icon_path(emulator: &str) -> PathBuf {
    let file_name = format!("{}.png", sanitize_file_part(emulator));
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.parent().map(|path| path.join("public").join("emulators").join(&file_name)),
        std::env::current_dir()
            .ok()
            .map(|path| path.join("public").join("emulators").join(&file_name)),
        Some(host_root().join("public").join("emulators").join(&file_name)),
        Some(host_root().join("gw-studio-tauri").join("public").join("emulators").join(&file_name)),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|path| path.exists())
        .unwrap_or_else(|| manifest_dir.join("..").join("public").join("emulators").join(file_name))
}

pub(crate) fn write_coverflow_source_image(source_image: &Path, destination: &Path) -> Result<(), String> {
    let bytes = fs::read(source_image)
        .map_err(|error| format!("failed to read preview image {}: {error}", source_image.display()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("failed to decode preview image {}: {error}", source_image.display()))?;
    let resized = image.resize(96, 96, FilterType::Lanczos3);
    let mut cursor = Cursor::new(Vec::new());
    resized
        .write_to(&mut cursor, ImageFormat::Jpeg)
        .map_err(|error| format!("failed to encode coverflow jpg: {error}"))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create cover image dir {}: {error}", parent.display()))?;
    }
    fs::write(destination, cursor.into_inner())
        .map_err(|error| format!("failed to write cover image {}: {error}", destination.display()))?;
    Ok(())
}
