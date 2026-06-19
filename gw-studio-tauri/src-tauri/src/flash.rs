use serde::Serialize;
use std::path::PathBuf;
use tauri::Emitter;

use crate::process_helpers::{build_backend_frequency_attempts, output_text};
use crate::pyocd_transport::run_pyocd_internal_flash_under_reset;
use crate::gnwmanager_transport::run_flash_command;
use crate::spi_helper::{
    run_gnwmanager_spi_erase_chunks, run_gnwmanager_spi_write,
};

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

    if target == "ext" {
        let full_flash_bytes = (external_flash_mb.max(1.0) * 1024.0 * 1024.0).round() as u64;
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
