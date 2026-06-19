use crate::paths::host_root;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Deserialize)]
pub(crate) struct StockBackupImportRequest {
    firmware_profile: String,
    kind: String,
    path: String,
}

#[derive(Serialize)]
pub(crate) struct StockBackupImportResult {
    name: String,
    path: String,
    size_bytes: u64,
}

pub(crate) fn stock_firmware_dir() -> PathBuf {
    host_root().join("StockFirmware")
}

pub(crate) fn bundle_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    format!("{}", now.as_secs())
}

pub(crate) fn firmware_profile_code(value: &str) -> &'static str {
    match value.trim().to_ascii_uppercase().as_str() {
        "Z" => "z",
        _ => "m",
    }
}

pub(crate) fn firmware_output_name(part: &str, profile_code: &str) -> String {
    match part {
        "spi" => format!("spi_{profile_code}.bin"),
        _ => format!("{part}{profile_code}.bin"),
    }
}

pub(crate) fn stock_firmware_output_name(part: &str, profile_code: &str) -> String {
    format!("stock_{}", firmware_output_name(part, profile_code))
}

pub(crate) fn explicit_stock_file(path: Option<&str>) -> Result<PathBuf, String> {
    let raw_path = path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "UI did not provide stock firmware path".to_string())?;
    let selected = PathBuf::from(raw_path);
    if !selected.is_file() {
        return Err(format!("UI stock firmware path is not a file: {}", selected.display()));
    }

    let stock_dir = stock_firmware_dir();
    fs::create_dir_all(&stock_dir)
        .map_err(|error| format!("failed to create StockFirmware dir: {error}"))?;
    let stock_dir = stock_dir
        .canonicalize()
        .map_err(|error| format!("failed to resolve StockFirmware dir: {error}"))?;
    let selected = selected
        .canonicalize()
        .map_err(|error| format!("failed to resolve UI stock firmware path: {error}"))?;
    if !selected.starts_with(&stock_dir) {
        return Err(format!(
            "UI stock firmware path must be inside StockFirmware folder {}: {}",
            stock_dir.display(),
            selected.display()
        ));
    }
    Ok(selected)
}

fn read_file_prefix(source: &Path, max_bytes: usize) -> Result<Vec<u8>, String> {
    let mut input = fs::File::open(source)
        .map_err(|error| format!("failed to open {}: {error}", source.display()))?;
    let mut data = vec![0_u8; max_bytes];
    input
        .read_exact(&mut data)
        .map_err(|error| format!("failed to read {} prefix: {error}", source.display()))?;
    Ok(data)
}

pub(crate) fn validate_stock_bank1_file(stock_bank1_path: &Path, profile_code: &str) -> Result<(), String> {
    let bank1 = read_file_prefix(stock_bank1_path, 128 * 1024)?;
    let bank1_crc = crc32fast::hash(&bank1);
    let expected_bank1_crc = if profile_code == "z" { 0x3420bccf } else { 0xefaaefe6 };
    if bank1_crc != expected_bank1_crc {
        return Err(format!(
            "Selected Bank1 is not a valid {} stock firmware for dualboot patch: {} (CRC32 {:08X}, expected {:08X})",
            profile_code.to_ascii_uppercase(),
            stock_bank1_path.display(),
            bank1_crc,
            expected_bank1_crc
        ));
    }
    Ok(())
}

pub(crate) fn detect_stock_bank1_profile(stock_bank1_path: &Path) -> Result<&'static str, String> {
    let bank1 = read_file_prefix(stock_bank1_path, 128 * 1024)?;
    match crc32fast::hash(&bank1) {
        0xefaaefe6 => Ok("m"),
        0x3420bccf => Ok("z"),
        crc => Err(format!(
            "Selected Bank1 is not a known original Mario/Zelda stock firmware: {} (CRC32 {:08X})",
            stock_bank1_path.display(),
            crc
        )),
    }
}

pub(crate) fn validate_stock_spi_file(stock_spi_path: &Path, profile_code: &str) -> Result<(), String> {
    let stock_spi = fs::read(stock_spi_path)
        .map_err(|error| format!("failed to read stock SPI {}: {error}", stock_spi_path.display()))?;
    let (spi_region, expected_spi_crc) = if profile_code == "z" {
        let start = 0x20000_usize;
        let end = 0x3254A0_usize;
        if stock_spi.len() < end {
            return Err(format!(
                "Selected SPI stock backup is too small for Zelda dualboot patch: {} bytes, need at least {} bytes",
                stock_spi.len(),
                end
            ));
        }
        (&stock_spi[start..end], 0x07a478d4_u32)
    } else {
        let strip_tail = 8192_usize;
        let min_len = (1024 * 1024) as usize;
        if stock_spi.len() < min_len {
            return Err(format!(
                "Selected SPI stock backup is too small for Mario dualboot patch: {} bytes, need at least {} bytes",
                stock_spi.len(),
                min_len
            ));
        }
        (&stock_spi[..min_len - strip_tail], 0x5f40d6bb_u32)
    };
    let spi_crc = crc32fast::hash(spi_region);
    if spi_crc != expected_spi_crc {
        return Err(format!(
            "Selected SPI is not a valid {} stock firmware for dualboot patch: {} (CRC32 {:08X}, expected {:08X})",
            profile_code.to_ascii_uppercase(),
            stock_spi_path.display(),
            spi_crc,
            expected_spi_crc
        ));
    }

    Ok(())
}

pub(crate) fn detect_stock_spi_profile(stock_spi_path: &Path) -> Result<&'static str, String> {
    let stock_spi = fs::read(stock_spi_path)
        .map_err(|error| format!("failed to read stock SPI {}: {error}", stock_spi_path.display()))?;

    if stock_spi.len() >= 1024 * 1024 {
        let mario_region = &stock_spi[..(1024 * 1024) - 8192];
        if crc32fast::hash(mario_region) == 0x5f40d6bb {
            return Ok("m");
        }
    }

    let zelda_end = 0x3254A0_usize;
    if stock_spi.len() >= zelda_end {
        let zelda_region = &stock_spi[0x20000..zelda_end];
        if crc32fast::hash(zelda_region) == 0x07a478d4 {
            return Ok("z");
        }
    }

    Err(format!(
        "Selected SPI is not a known original Mario/Zelda stock firmware: {}",
        stock_spi_path.display()
    ))
}

pub(crate) fn validate_patch_stock_inputs(stock_bank1_path: &Path, stock_spi_path: &Path, profile_code: &str) -> Result<(), String> {
    validate_stock_bank1_file(stock_bank1_path, profile_code)?;
    validate_stock_spi_file(stock_spi_path, profile_code)?;
    Ok(())
}

fn validate_stock_backup_size(part: &str, model_name: &str, size_bytes: u64) -> Result<(), String> {
    match part {
        "bank1" => {
            let min = 128 * 1024;
            let max = 1024 * 1024;
            if size_bytes < min || size_bytes > max {
                return Err(format!(
                    "Bank1 backup size must be between 128 KB and 1 MB, got {} bytes",
                    size_bytes
                ));
            }
        }
        "spi" => {
            let min = if model_name == "z" { 4 } else { 1 } * 1024 * 1024;
            if size_bytes < min {
                return Err(format!(
                    "SPI backup is too small for {} profile: got {} bytes, need at least {} bytes",
                    model_name.to_ascii_uppercase(),
                    size_bytes,
                    min
                ));
            }
        }
        _ => return Err(format!("unsupported stock backup part: {part}")),
    }
    Ok(())
}

pub(crate) fn resolve_model_name(model: &str) -> Result<&'static str, String> {
    let normalized_model = model.trim().to_ascii_lowercase();
    if normalized_model == "m" {
        Ok("m")
    } else if normalized_model == "z" {
        Ok("z")
    } else {
        Err(format!("unsupported backup model: {model}"))
    }
}

fn profile_display_name(model_name: &str) -> &'static str {
    if model_name == "z" {
        "Zelda"
    } else {
        "Mario"
    }
}

#[tauri::command]
pub(crate) fn import_stock_backup(request: StockBackupImportRequest) -> Result<StockBackupImportResult, String> {
    let requested_model_name = resolve_model_name(&request.firmware_profile)?;
    let kind = request.kind.trim().to_ascii_lowercase();
    let part = match kind.as_str() {
        "mcu" | "bank1" => "bank1",
        "spi" | "ext" | "extflash" => "spi",
        _ => return Err(format!("unsupported stock backup kind: {}", request.kind)),
    };

    let source = PathBuf::from(request.path.trim());
    if !source.exists() {
        return Err(format!("stock backup file not found: {}", source.display()));
    }
    if !source.is_file() {
        return Err(format!("stock backup path is not a file: {}", source.display()));
    }

    let size_bytes = fs::metadata(&source)
        .map_err(|error| format!("failed to stat stock backup file: {error}"))?
        .len();
    let detected_model_name = match part {
        "bank1" => detect_stock_bank1_profile(&source)?,
        "spi" => detect_stock_spi_profile(&source)?,
        _ => requested_model_name,
    };
    if detected_model_name != requested_model_name {
        return Err(format!(
            "Selected {} stock firmware is for {}, but current operation requires {}. Choose matching original {} stock files.",
            if part == "bank1" { "Bank1" } else { "SPI" },
            profile_display_name(detected_model_name),
            profile_display_name(requested_model_name),
            profile_display_name(requested_model_name)
        ));
    }
    match part {
        "bank1" => validate_stock_bank1_file(&source, requested_model_name)?,
        "spi" => validate_stock_spi_file(&source, requested_model_name)?,
        _ => {}
    }
    validate_stock_backup_size(part, detected_model_name, size_bytes)?;

    let stock_dir = stock_firmware_dir();
    fs::create_dir_all(&stock_dir)
        .map_err(|error| format!("failed to create stock firmware dir: {error}"))?;
    let destination = stock_dir.join(stock_firmware_output_name(part, detected_model_name));
    fs::copy(&source, &destination)
        .map_err(|error| format!("failed to import stock backup: {error}"))?;

    Ok(StockBackupImportResult {
        name: destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        path: destination.to_string_lossy().to_string(),
        size_bytes,
    })
}

pub(crate) fn mirror_backup_to_stock_name(source: &Path, part: &str, model_name: &str) -> Result<(), String> {
    let detected_model_name = match part {
        "bank1" => match detect_stock_bank1_profile(source) {
            Ok(profile) => profile,
            Err(_) => return Ok(()),
        },
        "spi" => match detect_stock_spi_profile(source) {
            Ok(profile) => profile,
            Err(_) => return Ok(()),
        },
        _ => model_name,
    };

    let destination_dir = stock_firmware_dir();
    fs::create_dir_all(&destination_dir)
        .map_err(|error| format!("failed to create stock firmware dir: {error}"))?;
    let destination = destination_dir.join(stock_firmware_output_name(part, detected_model_name));
    if source != destination {
        fs::copy(source, &destination)
            .map_err(|error| format!("failed to save stock-named backup {}: {error}", destination.display()))?;
    }
    Ok(())
}
