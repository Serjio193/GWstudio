use std::fs;
use std::path::PathBuf;

use crate::paths::workspace_root;

pub(crate) fn bundles_dir() -> PathBuf {
    workspace_root().join("bundles")
}

pub(crate) fn build_workspaces_dir() -> PathBuf {
    workspace_root().join("build_workspaces")
}

pub(crate) fn sanitize_file_part(value: &str) -> String {
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

pub(crate) fn copy_if_exists(source: &PathBuf, destination: &PathBuf) -> Result<bool, String> {
    if !source.exists() {
        return Ok(false);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("failed to create parent dir: {error}"))?;
    }
    fs::copy(source, destination).map_err(|error| format!("failed to copy file: {error}"))?;
    Ok(true)
}

pub(crate) fn retro_go_emulator_dir(emulator: &str) -> Option<&'static str> {
    match emulator {
        "nes" => Some("nes"),
        "gb" => Some("gb"),
        "gbc" => Some("gb"),
        "sms" => Some("sms"),
        "gg" => Some("gg"),
        "pce" => Some("pce"),
        "col" => Some("col"),
        "gw" => Some("gw"),
        "md" => Some("md"),
        "sg" => Some("sg"),
        "msx" => Some("msx"),
        "wsv" => Some("wsv"),
        "a7800" => Some("a7800"),
        "tama" => Some("tama"),
        _ => None,
    }
}

pub(crate) fn retro_go_workspace_rom_name(_emulator: &str, fallback_name: &str) -> String {
    sanitize_file_part(fallback_name)
}

pub(crate) fn ensure_clean_dir(path: &PathBuf) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| format!("failed to clear dir {}: {error}", path.display()))?;
    }
    fs::create_dir_all(path).map_err(|error| format!("failed to create dir {}: {error}", path.display()))?;
    Ok(())
}

pub(crate) fn copy_dir_filtered(source: &PathBuf, destination: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("failed to create dir {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source).map_err(|error| format!("failed to read dir {}: {error}", source.display()))? {
        let entry = entry.map_err(|error| format!("failed to read dir entry: {error}"))?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == ".git" || name_str == "build" {
            continue;
        }
        let dest_path = destination.join(&name);
        if path.is_dir() {
            copy_dir_filtered(&path, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create dir {}: {error}", parent.display()))?;
            }
            fs::copy(&path, &dest_path)
                .map_err(|error| format!("failed to copy {} -> {}: {error}", path.display(), dest_path.display()))?;
        }
    }
    Ok(())
}
