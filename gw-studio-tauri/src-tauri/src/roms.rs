use crate::paths::workspace_root;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub(crate) struct RomImportFile {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
pub(crate) struct RomImportRequest {
    emulator: String,
    files: Vec<RomImportFile>,
}

#[derive(Serialize)]
struct RomImportEntry {
    title: String,
    path: String,
    size_bytes: u64,
}

#[derive(Serialize)]
pub(crate) struct RomImportResult {
    entries: Vec<RomImportEntry>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct AutoRomImportEntry {
    emulator: String,
    title: String,
    path: String,
    size_bytes: u64,
}

#[derive(Serialize)]
pub(crate) struct AutoRomImportResult {
    entries: Vec<AutoRomImportEntry>,
    warnings: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct RomImportPathsRequest {
    paths: Vec<String>,
}

fn rom_imports_dir() -> PathBuf {
    workspace_root().join("rom_imports")
}

fn rom_extensions_for_emulator(emulator: &str) -> &'static [&'static str] {
    match emulator {
        "nes" => &[".nes", ".fds", ".nsf"],
        "gb" => &[".gb"],
        "gbc" => &[".gbc"],
        "sms" => &[".sms"],
        "gg" => &[".gg"],
        "pce" => &[".pce"],
        "col" => &[".col"],
        "gw" => &[".gw"],
        "md" => &[".md", ".bin", ".gen"],
        "sg" => &[".sg"],
        "msx" => &[".rom", ".mx1", ".mx2", ".dsk"],
        "wsv" => &[".sv", ".bin"],
        "a7800" => &[".a78", ".bin"],
        "tama" => &[".b"],
        _ => &[],
    }
}

fn emulator_ids() -> &'static [&'static str] {
    &[
        "nes", "gb", "gbc", "sms", "gg", "pce", "col", "gw", "md", "sg", "msx", "wsv",
        "a7800", "tama",
    ]
}

fn ambiguous_rom_extension(extension: &str) -> bool {
    matches!(extension, ".bin" | ".dsk" | ".sfc")
}

fn file_extension_lower(file_name: &str) -> Option<String> {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_ascii_lowercase()))
}

fn emulator_for_file_name(file_name: &str) -> Option<&'static str> {
    let lower_name = file_name.to_ascii_lowercase();
    let extension = file_extension_lower(file_name)?;

    if ambiguous_rom_extension(&extension) {
        return None;
    }

    for emulator in emulator_ids() {
        if rom_extensions_for_emulator(emulator)
            .iter()
            .any(|ext| lower_name.ends_with(ext))
        {
            return Some(emulator);
        }
    }
    None
}

fn is_ambiguous_rom_file_name(file_name: &str) -> bool {
    file_extension_lower(file_name)
        .as_deref()
        .map(ambiguous_rom_extension)
        .unwrap_or(false)
}

fn is_supported_import_name(file_name: &str) -> bool {
    let lower_name = file_name.to_ascii_lowercase();
    emulator_for_file_name(file_name).is_some()
        || is_ambiguous_rom_file_name(file_name)
        || lower_name.ends_with(".zip")
        || lower_name.ends_with(".7z")
}

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

fn write_imported_rom(emulator: &str, source_label: &str, file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let dest_dir = rom_imports_dir()
        .join(sanitize_file_part(emulator))
        .join(sanitize_file_part(source_label));
    fs::create_dir_all(&dest_dir).map_err(|error| format!("failed to create ROM import dir: {error}"))?;
    let dest_path = dest_dir.join(sanitize_file_part(file_name));
    fs::write(&dest_path, bytes).map_err(|error| format!("failed to write imported ROM: {error}"))?;
    Ok(dest_path)
}

fn import_auto_from_named_bytes(
    file_name: &str,
    bytes: Vec<u8>,
    entries: &mut Vec<AutoRomImportEntry>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let lower_name = file_name.to_ascii_lowercase();
    let source_label = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("import");

    if let Some(emulator) = emulator_for_file_name(file_name) {
        let dest_path = write_imported_rom(emulator, source_label, file_name, &bytes)?;
        let title = Path::new(file_name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(file_name)
            .to_string();
        entries.push(AutoRomImportEntry {
            emulator: emulator.to_string(),
            title,
            path: dest_path.to_string_lossy().to_string(),
            size_bytes: bytes.len() as u64,
        });
        return Ok(());
    }

    if is_ambiguous_rom_file_name(file_name) {
        let message = if file_extension_lower(file_name).as_deref() == Some(".sfc") {
            "SFC ROM skipped: SMW/Zelda3 direct loaders were removed from this build"
        } else {
            "Ambiguous ROM skipped. Add it with the + button for the target emulator."
        };
        warnings.push(format!("{message}: {file_name}"));
        return Ok(());
    }

    if lower_name.ends_with(".zip") {
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|error| format!("failed to open zip archive {}: {error}", file_name))?;

        for index in 0..archive.len() {
            let mut item = archive
                .by_index(index)
                .map_err(|error| format!("failed to read zip entry in {}: {error}", file_name))?;
            if item.is_dir() {
                continue;
            }
            let entry_name = item.name().replace('\\', "/");
            let Some(emulator) = emulator_for_file_name(&entry_name) else {
                if is_ambiguous_rom_file_name(&entry_name) {
                    let message = if file_extension_lower(&entry_name).as_deref() == Some(".sfc") {
                        "SFC ROM skipped: SMW/Zelda3 direct loaders were removed from this build"
                    } else {
                        "Ambiguous ROM skipped. Add it with the + button for the target emulator."
                    };
                    warnings.push(format!("{message} In {file_name}: {entry_name}"));
                }
                continue;
            };
            let base_name = Path::new(&entry_name)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(&entry_name)
                .to_string();
            let mut content = Vec::new();
            item.read_to_end(&mut content)
                .map_err(|error| format!("failed to extract zip entry {entry_name}: {error}"))?;
            let dest_path = write_imported_rom(emulator, source_label, &base_name, &content)?;
            let title = Path::new(&base_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or(&base_name)
                .to_string();
            entries.push(AutoRomImportEntry {
                emulator: emulator.to_string(),
                title,
                path: dest_path.to_string_lossy().to_string(),
                size_bytes: content.len() as u64,
            });
        }
        return Ok(());
    }

    if lower_name.ends_with(".7z") {
        warnings.push(format!(
            "7z archive is not supported yet: {}. Repack to ZIP or extract manually.",
            file_name
        ));
        return Ok(());
    }

    warnings.push(format!("Unsupported file skipped: {}", file_name));
    Ok(())
}

#[tauri::command]
pub(crate) fn import_rom_files(request: RomImportRequest) -> Result<RomImportResult, String> {
    let extensions = rom_extensions_for_emulator(&request.emulator);
    if extensions.is_empty() {
        return Err(format!("unsupported emulator: {}", request.emulator));
    }

    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for file in request.files {
        let lower_name = file.name.to_ascii_lowercase();
        let source_label = Path::new(&file.name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("import");

        if extensions.iter().any(|ext| lower_name.ends_with(ext)) {
            let dest_path = write_imported_rom(&request.emulator, source_label, &file.name, &file.bytes)?;
            let title = Path::new(&file.name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or(&file.name)
                .to_string();
            entries.push(RomImportEntry {
                title,
                path: dest_path.to_string_lossy().to_string(),
                size_bytes: file.bytes.len() as u64,
            });
            continue;
        }

        if lower_name.ends_with(".zip") {
            let reader = Cursor::new(file.bytes);
            let mut archive = zip::ZipArchive::new(reader)
                .map_err(|error| format!("failed to open zip archive {}: {error}", file.name))?;

            for index in 0..archive.len() {
                let mut item = archive
                    .by_index(index)
                    .map_err(|error| format!("failed to read zip entry in {}: {error}", file.name))?;
                if item.is_dir() {
                    continue;
                }
                let entry_name = item.name().replace('\\', "/");
                let entry_lower = entry_name.to_ascii_lowercase();
                if !extensions.iter().any(|ext| entry_lower.ends_with(ext)) {
                    continue;
                }
                let base_name = Path::new(&entry_name)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&entry_name)
                    .to_string();
                let mut content = Vec::new();
                item.read_to_end(&mut content)
                    .map_err(|error| format!("failed to extract zip entry {entry_name}: {error}"))?;
                let dest_path = write_imported_rom(&request.emulator, source_label, &base_name, &content)?;
                let title = Path::new(&base_name)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&base_name)
                    .to_string();
                entries.push(RomImportEntry {
                    title,
                    path: dest_path.to_string_lossy().to_string(),
                    size_bytes: content.len() as u64,
                });
            }
            continue;
        }

        if lower_name.ends_with(".7z") {
            warnings.push(format!(
                "7z archive is not supported yet: {}. Repack to ZIP or extract manually.",
                file.name
            ));
            continue;
        }

        warnings.push(format!("Unsupported file skipped: {}", file.name));
    }

    Ok(RomImportResult { entries, warnings })
}

#[tauri::command]
pub(crate) fn import_rom_files_auto(files: Vec<RomImportFile>) -> Result<AutoRomImportResult, String> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for file in files {
        import_auto_from_named_bytes(&file.name, file.bytes, &mut entries, &mut warnings)?;
    }

    Ok(AutoRomImportResult { entries, warnings })
}

#[tauri::command]
pub(crate) fn import_rom_paths_auto(request: RomImportPathsRequest) -> Result<AutoRomImportResult, String> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for path in request.paths {
        let path_buf = PathBuf::from(&path);
        let file_name = path_buf
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&path)
            .to_string();
        if !path_buf.is_file() {
            warnings.push(format!("Dropped path ignored (not a file): {}", path));
            continue;
        }
        if !is_supported_import_name(&file_name) {
            warnings.push(format!("Dropped file ignored: {}", file_name));
            continue;
        }
        let bytes = fs::read(&path).map_err(|error| format!("failed to read dropped file {}: {error}", path))?;
        import_auto_from_named_bytes(&file_name, bytes, &mut entries, &mut warnings)?;
    }

    Ok(AutoRomImportResult { entries, warnings })
}
