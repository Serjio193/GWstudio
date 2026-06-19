use crate::paths::workspace_root;
use crate::stock::{
    firmware_profile_code, stock_firmware_dir, validate_stock_bank1_file, validate_stock_spi_file,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub(crate) struct DeviceBackupLookupRequest {
    device_uid: String,
    firmware_profile: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct DeviceBackupLookupResult {
    mcu_name: String,
    mcu_path: String,
    bank2_name: String,
    bank2_path: String,
    spi_name: String,
    spi_path: String,
}

pub(crate) fn backups_dir() -> PathBuf {
    workspace_root().join("backups")
}

pub(crate) fn device_backups_dir(uid: &str) -> PathBuf {
    backups_dir().join(uid)
}

pub(crate) fn legacy_device_backups_dir(uid: &str) -> PathBuf {
    workspace_root().join("devices").join(uid).join("backups")
}

pub(crate) fn newest_matching_file(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(prefix) && name.ends_with(".bin"))
                .unwrap_or(false)
        })
        .collect();

    matches.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    matches.pop()
}

pub(crate) fn matching_files(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(prefix) && name.ends_with(".bin"))
                .unwrap_or(false)
        })
        .collect();

    matches.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    matches.reverse();
    matches
}

pub(crate) fn newest_named_backup_file(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.is_file())
        .max_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok())
}

fn newest_restore_bank1_file(dir: &Path, legacy_dir: &Path) -> Option<PathBuf> {
    newest_matching_file(dir, "internal_flash_bank1_backup_")
        .or_else(|| newest_matching_file(dir, "internal_flash_backup_"))
        .or_else(|| newest_matching_file(dir, "bank1"))
        .or_else(|| newest_matching_file(legacy_dir, "internal_flash_bank1_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "internal_flash_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "bank1"))
}

fn newest_restore_bank2_file(dir: &Path, legacy_dir: &Path) -> Option<PathBuf> {
    newest_matching_file(dir, "internal_flash_bank2_backup_")
        .or_else(|| newest_matching_file(dir, "bank2"))
        .or_else(|| newest_matching_file(legacy_dir, "internal_flash_bank2_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "bank2"))
}

fn newest_restore_spi_file(dir: &Path, legacy_dir: &Path) -> Option<PathBuf> {
    newest_matching_file(dir, "flash_backup_")
        .or_else(|| newest_matching_file(dir, "spi_backup_"))
        .or_else(|| newest_named_backup_file(dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"]))
        .or_else(|| newest_matching_file(dir, "spi_"))
        .or_else(|| newest_matching_file(legacy_dir, "flash_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "spi_backup_"))
        .or_else(|| newest_named_backup_file(legacy_dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"]))
        .or_else(|| newest_matching_file(legacy_dir, "spi_"))
}

fn newest_valid_stock_bank1_file(dir: &Path, legacy_dir: &Path, profile_code: &str) -> Option<PathBuf> {
    [dir, legacy_dir]
        .into_iter()
        .flat_map(|candidate_dir| matching_files(candidate_dir, "stock_bank1"))
        .find(|path| validate_stock_bank1_file(path, profile_code).is_ok())
}

fn newest_valid_stock_spi_file(dir: &Path, legacy_dir: &Path, profile_code: &str) -> Option<PathBuf> {
    [dir, legacy_dir]
        .into_iter()
        .flat_map(|candidate_dir| matching_files(candidate_dir, "stock_spi"))
        .find(|path| validate_stock_spi_file(path, profile_code).is_ok())
}

pub(crate) fn backup_ready_for_device(device_uid: &str) -> bool {
    let uid = device_uid.trim();
    if uid.is_empty() || uid.eq_ignore_ascii_case("UNKNOWN") {
        return false;
    }

    let dir = device_backups_dir(uid);
    let legacy_dir = legacy_device_backups_dir(uid);
    let mcu = newest_matching_file(&dir, "bank1")
        .or_else(|| newest_matching_file(&dir, "internal_flash_bank1_backup_"))
        .or_else(|| newest_matching_file(&dir, "internal_flash_backup_"))
        .or_else(|| newest_matching_file(&legacy_dir, "bank1"))
        .or_else(|| newest_matching_file(&legacy_dir, "internal_flash_bank1_backup_"))
        .or_else(|| newest_matching_file(&legacy_dir, "internal_flash_backup_"));
    let spi = newest_named_backup_file(&dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"])
        .or_else(|| newest_matching_file(&dir, "spi_"))
        .or_else(|| newest_matching_file(&dir, "flash_backup_"))
        .or_else(|| newest_named_backup_file(&legacy_dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"]))
        .or_else(|| newest_matching_file(&legacy_dir, "spi_"))
        .or_else(|| newest_matching_file(&legacy_dir, "flash_backup_"));

    mcu.is_some() && spi.is_some()
}

fn backup_lookup_result(mcu: Option<PathBuf>, bank2: Option<PathBuf>, spi: Option<PathBuf>) -> DeviceBackupLookupResult {
    DeviceBackupLookupResult {
        mcu_name: mcu
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        mcu_path: mcu
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        bank2_name: bank2
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        bank2_path: bank2
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        spi_name: spi
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        spi_path: spi
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    }
}

#[tauri::command]
pub(crate) fn lookup_device_backups(request: DeviceBackupLookupRequest) -> Result<DeviceBackupLookupResult, String> {
    let uid = request.device_uid.trim();
    if uid.is_empty() || uid.eq_ignore_ascii_case("UNKNOWN") {
        return Ok(backup_lookup_result(None, None, None));
    }

    let dir = device_backups_dir(uid);
    let legacy_dir = legacy_device_backups_dir(uid);
    let mcu = newest_restore_bank1_file(&dir, &legacy_dir);
    let bank2 = newest_restore_bank2_file(&dir, &legacy_dir);
    let spi = newest_restore_spi_file(&dir, &legacy_dir);

    Ok(backup_lookup_result(mcu, bank2, spi))
}

#[tauri::command]
pub(crate) fn lookup_stock_backups(request: DeviceBackupLookupRequest) -> Result<DeviceBackupLookupResult, String> {
    let dir = stock_firmware_dir();
    let legacy_dir = dir.clone();
    let profile_code = request
        .firmware_profile
        .as_deref()
        .map(firmware_profile_code)
        .unwrap_or("m");
    let mcu = newest_valid_stock_bank1_file(&dir, &legacy_dir, profile_code);
    let bank2: Option<PathBuf> = None;
    let spi = newest_valid_stock_spi_file(&dir, &legacy_dir, profile_code);

    Ok(backup_lookup_result(mcu, bank2, spi))
}

#[tauri::command]
pub(crate) fn lookup_restore_backups(request: DeviceBackupLookupRequest) -> Result<DeviceBackupLookupResult, String> {
    let uid = request.device_uid.trim();
    if uid.is_empty() || uid.eq_ignore_ascii_case("UNKNOWN") {
        return Ok(backup_lookup_result(None, None, None));
    }

    let dir = device_backups_dir(uid);
    let legacy_dir = legacy_device_backups_dir(uid);
    let mcu = newest_restore_bank1_file(&dir, &legacy_dir);
    let bank2 = newest_restore_bank2_file(&dir, &legacy_dir);
    let spi = newest_restore_spi_file(&dir, &legacy_dir);

    Ok(backup_lookup_result(mcu, bank2, spi))
}
