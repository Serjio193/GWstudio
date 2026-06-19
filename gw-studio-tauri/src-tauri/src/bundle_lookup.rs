use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::paths::workspace_root;
use crate::stock::firmware_profile_code;

#[derive(Serialize)]
pub(crate) struct FirmwareBundleLookupResult {
    found: bool,
    message: String,
    bundle_dir: String,
    manifest_path: String,
    bank1_candidate_path: String,
    bank2_candidate_path: String,
    extflash_build_path: String,
    extflash_build_size_bytes: u64,
}

fn bundles_dir() -> std::path::PathBuf {
    workspace_root().join("bundles")
}

fn build_workspaces_dir() -> std::path::PathBuf {
    workspace_root().join("build_workspaces")
}

fn latest_bank1_patch_failure_message() -> Option<String> {
    let root = build_workspaces_dir();
    if !root.exists() {
        return None;
    }
    let mut logs = Vec::new();
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path().join("build_gw_studio_patch.log");
        if path.exists() {
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            logs.push((modified, path));
        }
    }
    logs.sort_by(|a, b| b.0.cmp(&a.0));
    let path = logs.first()?.1.clone();
    let text = fs::read_to_string(&path).ok()?;
    let tail = text
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("{}\n{}", path.display(), tail))
}

#[tauri::command]
pub(crate) fn latest_firmware_bundle() -> Result<FirmwareBundleLookupResult, String> {
    let root = bundles_dir();
    if !root.exists() {
        return Ok(FirmwareBundleLookupResult {
            found: false,
            message: "No firmware bundle directory exists. Run Build Firmware first.".to_string(),
            bundle_dir: String::new(),
            manifest_path: String::new(),
            bank1_candidate_path: String::new(),
            bank2_candidate_path: String::new(),
            extflash_build_path: String::new(),
            extflash_build_size_bytes: 0,
        });
    }

    let mut manifests = Vec::new();
    for entry in fs::read_dir(&root).map_err(|error| format!("failed to read bundles dir: {error}"))? {
        let entry = entry.map_err(|error| format!("failed to read bundle dir entry: {error}"))?;
        let path = entry.path().join("bundle_manifest.json");
        if path.exists() {
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            manifests.push((modified, path));
        }
    }
    manifests.sort_by(|a, b| b.0.cmp(&a.0));
    let mut skipped_reason = String::new();

    for (_, manifest_path) in manifests {
        let manifest_text = match fs::read_to_string(&manifest_path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let manifest: serde_json::Value = match serde_json::from_str(&manifest_text) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let path_field = |name: &str| -> String {
            manifest
                .get(name)
                .and_then(|value| value.as_str())
                .filter(|value| Path::new(value).is_file())
                .unwrap_or("")
                .to_string()
        };
        let bank1 = path_field("bank1_candidate_path");
        let bank2 = path_field("bank2_candidate_path");
        let spi = path_field("extflash_build_path");
        let spi_full_image = manifest
            .get("spi_full_image")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let has_spi_prefix_source = manifest
            .get("spi_prefix_source_path")
            .and_then(|value| value.as_str())
            .filter(|value| Path::new(value).is_file())
            .is_some();
        let firmware_code = manifest
            .get("firmware_code")
            .and_then(|value| value.as_str())
            .map(firmware_profile_code)
            .unwrap_or("m");
        let bank1_is_dualboot = manifest
            .get("bank1_dualboot")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            || Path::new(&bank1)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("bank1d.bin"))
                .unwrap_or(false);
        if bank1.is_empty() || bank2.is_empty() || spi.is_empty() || !bank1_is_dualboot {
            if skipped_reason.is_empty() {
                skipped_reason = format!(
                    "Latest bundle is not flashable for dualboot: Bank1={}, Bank2={}, SPI={}, dualboot={}. Rebuild must produce bank1d.bin.",
                    if bank1.is_empty() { "missing" } else { "present" },
                    if bank2.is_empty() { "missing" } else { "present" },
                    if spi.is_empty() { "missing" } else { "present" },
                    bank1_is_dualboot,
                );
            }
            continue;
        }
        if !spi_full_image || !has_spi_prefix_source {
            if skipped_reason.is_empty() {
                skipped_reason = "Latest bundle uses old SPI image format. Rebuild firmware to create a patched full flashable SPI image.".to_string();
            }
            continue;
        }
        let bank1_size = fs::metadata(&bank1).map(|metadata| metadata.len()).unwrap_or(0);
        let expected_bank1_size = if firmware_code == "z" { 128 * 1024 } else { 256 * 1024 };
        if bank1_size != expected_bank1_size {
            if skipped_reason.is_empty() {
                skipped_reason = format!(
                    "Latest bundle has invalid Bank1 size for {}: {} bytes, expected {} bytes.",
                    firmware_code.to_ascii_uppercase(),
                    bank1_size,
                    expected_bank1_size,
                );
            }
            continue;
        }
        let extflash_build_size_bytes = if spi.is_empty() {
            0
        } else {
            fs::metadata(&spi).map(|metadata| metadata.len()).unwrap_or(0)
        };
        let bundle_dir = manifest
            .get("bundle_dir")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        return Ok(FirmwareBundleLookupResult {
            found: true,
            message: "Dualboot firmware bundle found.".to_string(),
            bundle_dir,
            manifest_path: manifest_path.to_string_lossy().to_string(),
            bank1_candidate_path: bank1,
            bank2_candidate_path: bank2,
            extflash_build_path: spi,
            extflash_build_size_bytes,
        });
    }

    let message = if skipped_reason.is_empty() {
        latest_bank1_patch_failure_message()
            .unwrap_or_else(|| "No dualboot firmware bundle found. Run Build Firmware first.".to_string())
    } else if let Some(patch_failure) = latest_bank1_patch_failure_message() {
        format!("{skipped_reason}\nLatest Bank1 patch error:\n{patch_failure}")
    } else {
        skipped_reason
    };

    Ok(FirmwareBundleLookupResult {
        found: false,
        message,
        bundle_dir: String::new(),
        manifest_path: String::new(),
        bank1_candidate_path: String::new(),
        bank2_candidate_path: String::new(),
        extflash_build_path: String::new(),
        extflash_build_size_bytes: 0,
    })
}
