use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;

use crate::process_helpers::{build_backend_frequency_attempts, output_text};
use crate::pyocd_transport::run_pyocd_internal_flash_under_reset;
use crate::gnwmanager_transport::run_flash_command;
use crate::spi_helper::{
    run_gnwmanager_spi_erase_chunks, run_gnwmanager_spi_write,
};

const INTERNAL_BANK_MAX_BYTES: u64 = 1024 * 1024;
const SPI_MAX_SUPPORTED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Deserialize)]
struct BundleManifest {
    firmware_code: Option<String>,
    firmware_artifacts: Option<Vec<FirmwareArtifact>>,
}

#[derive(Deserialize)]
struct FirmwareArtifact {
    target: String,
    path: String,
    file_name: Option<String>,
    size_bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
pub(crate) struct FirmwareWriteResult {
    summary: String,
    path: String,
    target: String,
    backend: String,
    frequency: u32,
    stderr: String,
}

#[derive(Clone, Serialize)]
struct FirmwareWriteProgressEvent {
    phase: String,
    stage: String,
    progress: f64,
    speed_bps: f64,
    frequency: u32,
    backend: String,
    message: String,
}

pub(crate) fn emit_firmware_write_progress(
    app: &tauri::AppHandle,
    phase: &str,
    stage: &str,
    progress: f64,
    speed_bps: f64,
    frequency: u32,
    backend: &str,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "firmware-write-progress",
        FirmwareWriteProgressEvent {
            phase: phase.to_string(),
            stage: stage.to_string(),
            progress,
            speed_bps,
            frequency,
            backend: backend.to_string(),
            message: message.into(),
        },
    );
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn is_stm32h7_ram_address(value: u32) -> bool {
    (0x2000_0000..=0x2002_0000).contains(&value)
        || (0x2400_0000..=0x2408_0000).contains(&value)
        || (0x3000_0000..=0x3008_0000).contains(&value)
        || (0x3800_0000..=0x3801_0000).contains(&value)
}

fn is_stm32h7_flash_address(value: u32) -> bool {
    let normalized = value & !1;
    (0x0800_0000..0x0820_0000).contains(&normalized)
}

fn validate_bin_extension(source: &Path, target: &str) -> Result<(), String> {
    let extension_ok = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("bin"))
        .unwrap_or(false);
    if !extension_ok {
        return Err(format!(
            "{target} firmware must be a .bin file: {}",
            source.display()
        ));
    }
    Ok(())
}

fn validate_internal_flash_image(source: &Path, target: &str, size: u64) -> Result<(), String> {
    if size < 8 {
        return Err(format!(
            "{target} firmware is too small: {size} bytes"
        ));
    }
    if size > INTERNAL_BANK_MAX_BYTES {
        return Err(format!(
            "{target} firmware is too large: {size} bytes, max {INTERNAL_BANK_MAX_BYTES} bytes"
        ));
    }
    if size % 4 != 0 {
        return Err(format!(
            "{target} firmware size must be 4-byte aligned, got {size} bytes"
        ));
    }

    let header = fs::read(source)
        .map_err(|error| format!("failed to read {target} firmware header: {error}"))?;
    let initial_sp = read_u32_le(&header, 0)
        .ok_or_else(|| format!("failed to read {target} initial stack pointer"))?;
    let reset_handler = read_u32_le(&header, 4)
        .ok_or_else(|| format!("failed to read {target} reset handler"))?;
    if !is_stm32h7_ram_address(initial_sp) || !is_stm32h7_flash_address(reset_handler) {
        return Err(format!(
            "{target} firmware does not look like a STM32H7 internal flash image: SP=0x{initial_sp:08x}, Reset=0x{reset_handler:08x}"
        ));
    }
    Ok(())
}

fn validate_spi_flash_image(
    source: &Path,
    size: u64,
    external_flash_mb: f64,
    external_flash_offset_bytes: u64,
) -> Result<(), String> {
    if size == 0 {
        return Err("SPI firmware is empty".to_string());
    }
    if !external_flash_mb.is_finite() || external_flash_mb <= 0.0 {
        return Err(format!(
            "invalid external flash size: {external_flash_mb} MB"
        ));
    }
    let full_flash_bytes = (external_flash_mb * 1024.0 * 1024.0).round() as u64;
    if full_flash_bytes == 0 || full_flash_bytes > SPI_MAX_SUPPORTED_BYTES {
        return Err(format!(
            "unsupported external flash size: {full_flash_bytes} bytes"
        ));
    }
    if external_flash_offset_bytes > full_flash_bytes {
        return Err(format!(
            "SPI offset is outside flash capacity: offset={external_flash_offset_bytes}, capacity={full_flash_bytes}"
        ));
    }
    let end = external_flash_offset_bytes
        .checked_add(size)
        .ok_or_else(|| "SPI write range overflow".to_string())?;
    if end > full_flash_bytes {
        return Err(format!(
            "SPI image does not fit flash: offset={external_flash_offset_bytes}, size={size}, capacity={full_flash_bytes}, file={}",
            source.display()
        ));
    }
    if external_flash_offset_bytes % 4096 != 0 {
        return Err(format!(
            "SPI offset must be 4 KB aligned, got {external_flash_offset_bytes}"
        ));
    }
    Ok(())
}

fn validate_firmware_source(
    source: &Path,
    target: &str,
    external_flash_mb: f64,
    external_flash_offset_bytes: u64,
) -> Result<u64, String> {
    validate_bin_extension(source, target)?;
    let metadata = fs::metadata(source)
        .map_err(|error| format!("failed to stat firmware file {}: {error}", source.display()))?;
    let size = metadata.len();
    if target == "ext" {
        validate_spi_flash_image(source, size, external_flash_mb, external_flash_offset_bytes)?;
    } else if target == "bank1" || target == "bank2" {
        validate_internal_flash_image(source, target, size)?;
    } else {
        return Err(format!("unsupported flash target: {target}"));
    }
    Ok(size)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read firmware file {} for SHA256: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn canonical_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn expected_manifest_target(target: &str) -> &str {
    if target == "ext" {
        "spi"
    } else {
        target
    }
}

fn bundle_manifest_path_for_firmware(source: &Path) -> Option<PathBuf> {
    let firmware_dir = source.parent()?;
    if !firmware_dir
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("firmware"))
        .unwrap_or(false)
    {
        return None;
    }
    let bundle_dir = firmware_dir.parent()?;
    let manifest_path = bundle_dir.join("bundle_manifest.json");
    manifest_path.exists().then_some(manifest_path)
}

fn artifact_path_matches(source: &Path, artifact: &FirmwareArtifact) -> bool {
    let source_canonical = canonical_path(source);
    let artifact_path = PathBuf::from(&artifact.path);
    canonical_path(&artifact_path) == source_canonical
        || artifact
            .file_name
            .as_deref()
            .map(|name| {
                source
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(|source_name| source_name.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
}

fn validate_bundle_manifest_artifact(source: &Path, target: &str, size: u64) -> Result<(), String> {
    let Some(manifest_path) = bundle_manifest_path_for_firmware(source) else {
        return Ok(());
    };
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read bundle manifest {}: {error}", manifest_path.display()))?;
    let manifest: BundleManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("failed to parse bundle manifest {}: {error}", manifest_path.display()))?;
    let artifacts = manifest.firmware_artifacts.ok_or_else(|| {
        format!(
            "bundle manifest {} does not contain firmware_artifacts; rebuild firmware before flashing this bundle",
            manifest_path.display()
        )
    })?;
    let expected_target = expected_manifest_target(target);
    let artifact = artifacts
        .iter()
        .find(|artifact| artifact.target.eq_ignore_ascii_case(expected_target) && artifact_path_matches(source, artifact))
        .ok_or_else(|| {
            format!(
                "firmware file {} is not listed as {expected_target} in bundle manifest {}; rebuild firmware before flashing",
                source.display(),
                manifest_path.display()
            )
        })?;

    if artifact.size_bytes != size {
        return Err(format!(
            "bundle manifest size mismatch for {}: manifest={}, actual={}",
            source.display(),
            artifact.size_bytes,
            size
        ));
    }

    let expected_hash = artifact.sha256.trim().to_ascii_lowercase();
    if expected_hash.len() != 64 || !expected_hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!(
            "bundle manifest has invalid SHA256 for {}",
            source.display()
        ));
    }
    let actual_hash = sha256_file(source)?;
    if actual_hash != expected_hash {
        return Err(format!(
            "bundle manifest SHA256 mismatch for {}: expected {}, got {}",
            source.display(),
            expected_hash,
            actual_hash
        ));
    }

    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if expected_target == "bank1" && file_name != "bank1d.bin" {
        return Err(format!("bundle Bank1 artifact must be bank1d.bin, got {}", source.display()));
    }
    if expected_target == "bank2" && !file_name.starts_with("bank2") {
        return Err(format!("bundle Bank2 artifact name is unexpected: {}", source.display()));
    }
    if expected_target == "spi" && !file_name.starts_with("spi_") {
        return Err(format!("bundle SPI artifact name is unexpected: {}", source.display()));
    }

    if let Some(firmware_code) = manifest.firmware_code.as_deref() {
        let firmware_code = firmware_code.to_ascii_lowercase();
        if expected_target == "bank2" && !file_name.contains(&firmware_code) {
            return Err(format!(
                "bundle Bank2 artifact does not match firmware profile {}: {}",
                firmware_code,
                source.display()
            ));
        }
        if expected_target == "spi" && !file_name.contains(&firmware_code) {
            return Err(format!(
                "bundle SPI artifact does not match firmware profile {}: {}",
                firmware_code,
                source.display()
            ));
        }
    }

    Ok(())
}

fn run_single_flash_phase(
    app: &tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    source_path: String,
    target: &str,
    external_flash_mb: f64,
    external_flash_offset_bytes: u64,
) -> Result<FirmwareWriteResult, String> {
    let is_locked = protection.trim().eq_ignore_ascii_case("LOCKED");
    if is_locked {
        return Err("device is locked; flash write is unavailable while protection is enabled".to_string());
    }

    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Err(format!("firmware file not found: {}", source.display()));
    }
    if !source.is_file() {
        return Err(format!("firmware path is not a file: {}", source.display()));
    }
    let source_size = validate_firmware_source(&source, target, external_flash_mb, external_flash_offset_bytes)?;
    validate_bundle_manifest_artifact(&source, target, source_size)?;

    if target == "ext" {
        let full_flash_bytes = (external_flash_mb * 1024.0 * 1024.0).round() as u64;
        run_gnwmanager_spi_erase_chunks(app, frequency, full_flash_bytes)?;
        let output = run_gnwmanager_spi_write(app, frequency, &source, external_flash_offset_bytes)?;
        let text = output_text(&output);
        if output.status.success() {
            return Ok(FirmwareWriteResult {
                summary: format!("SPI flash completed successfully (gnwmanager, freq {frequency})"),
                path: source.to_string_lossy().to_string(),
                target: target.to_string(),
                backend: "gnwmanager".to_string(),
                frequency,
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        return Err(format!(
            "failed to flash SPI with GNWManager: {}",
            text.lines().last().unwrap_or("unknown error")
        ));
    }

    let used_backend = backend;

    if used_backend.eq_ignore_ascii_case("pyocd") && (target == "bank1" || target == "bank2") {
        let direct_bank = if target == "bank1" { 1_u8 } else { 2_u8 };
        let mut direct_frequencies = Vec::new();
        for value in [1_000_000_u32, 500_000, 240_000, 100_000, frequency] {
            if value > 0 && !direct_frequencies.contains(&value) {
                direct_frequencies.push(value);
            }
        }
        for direct_frequency in direct_frequencies {
            let output = run_pyocd_internal_flash_under_reset(
                app,
                target,
                direct_bank,
                &source,
                direct_frequency,
            )?;
            let text = output_text(&output);
            if output.status.success() {
                return Ok(FirmwareWriteResult {
                    summary: format!("{target} flash completed successfully (pyocd under-reset, freq {direct_frequency})"),
                    path: source.to_string_lossy().to_string(),
                    target: target.to_string(),
                    backend: "pyocd".to_string(),
                    frequency: direct_frequency,
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
            emit_firmware_write_progress(
                app,
                target,
                "write",
                0.0,
                0.0,
                direct_frequency,
                "pyocd",
                format!("Under-reset flash failed, trying fallback: {}", text.lines().last().unwrap_or("unknown error")),
            );
        }
    }

    let frequency_attempts = build_backend_frequency_attempts(&used_backend, frequency);
    let mut last_text = String::new();

    for candidate_frequency in frequency_attempts {
        let selected_frequency = candidate_frequency;
        let output = run_flash_command(&used_backend, selected_frequency, target, &source)?;
        let text = output_text(&output);
        if output.status.success() {
            return Ok(FirmwareWriteResult {
                summary: format!("{target} flash completed successfully ({used_backend}, freq {selected_frequency})"),
                path: source.to_string_lossy().to_string(),
                target: target.to_string(),
                backend: used_backend,
                frequency: selected_frequency,
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        last_text = text;
    }

    Err(format!(
        "failed to flash {target}: {}",
        last_text.lines().last().unwrap_or("unknown error")
    ))
}

#[tauri::command]
pub(crate) async fn write_bank1_firmware(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    path: String,
) -> Result<FirmwareWriteResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_flash_phase(&app, backend, frequency, protection, path, "bank1", 0.0, 0)
    })
    .await
    .map_err(|error| format!("failed to join bank1 flash task: {error}"))?
}

#[tauri::command]
pub(crate) async fn write_bank2_firmware(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    path: String,
) -> Result<FirmwareWriteResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_flash_phase(&app, backend, frequency, protection, path, "bank2", 0.0, 0)
    })
    .await
    .map_err(|error| format!("failed to join bank2 flash task: {error}"))?
}

#[tauri::command]
pub(crate) async fn write_spi_firmware(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    path: String,
    external_flash_mb: f64,
    external_flash_offset_bytes: u64,
) -> Result<FirmwareWriteResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_flash_phase(&app, backend, frequency, protection, path, "ext", external_flash_mb, external_flash_offset_bytes)
    })
    .await
    .map_err(|error| format!("failed to join SPI flash task: {error}"))?
}
