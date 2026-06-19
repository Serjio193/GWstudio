use serde::Serialize;
use crate::stock::{firmware_output_name, mirror_backup_to_stock_name, resolve_model_name};
use crate::device_state::required_active_backups_dir;
use crate::gnwmanager_transport::run_dump_with_progress;
use crate::process_helpers::{build_backend_frequency_attempts, output_text};
use crate::pyocd_transport::run_pyocd_internal_dump_under_reset;
use crate::spi_helper::run_gnwmanager_spi_read;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::Emitter;

#[derive(Serialize)]
pub(crate) struct BackupReadResult {
    pub(crate) summary: String,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) backend: String,
    pub(crate) phase: String,
    pub(crate) frequency: u32,
    pub(crate) speed_bps: f64,
    pub(crate) stderr: String,
}

#[derive(Clone, Serialize)]
struct BackupProgressEvent {
    phase: String,
    phase_progress: f64,
    total_progress: f64,
    speed_bps: f64,
    frequency: u32,
    backend: String,
    message: String,
}

#[derive(Clone, Serialize)]
struct BackupDebugEvent {
    phase: String,
    line: String,
    source: String,
}

pub(crate) fn emit_backup_progress(
    app: &tauri::AppHandle,
    phase: &str,
    phase_progress: f64,
    total_progress: f64,
    speed_bps: f64,
    frequency: u32,
    backend: &str,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "backup-progress",
        BackupProgressEvent {
            phase: phase.to_string(),
            phase_progress,
            total_progress,
            speed_bps,
            frequency,
            backend: backend.to_string(),
            message: message.into(),
        },
    );
}

pub(crate) fn emit_backup_debug(app: &tauri::AppHandle, phase: &str, source: &str, line: &str) {
    let _ = app.emit(
        "backup-debug",
        BackupDebugEvent {
            phase: phase.to_string(),
            line: line.to_string(),
            source: source.to_string(),
        },
    );
}

pub(crate) fn emit_backup_progress_throttled(
    app: &tauri::AppHandle,
    phase: &str,
    phase_progress: f64,
    total_progress: f64,
    speed_bps: f64,
    frequency: u32,
    backend: &str,
    message: impl Into<String>,
    last_emit_at: &mut Instant,
    last_phase_progress: &mut f64,
    last_message: &mut String,
    force: bool,
) {
    let message = message.into();
    let progress_changed = (phase_progress - *last_phase_progress).abs() >= 1.0;
    let message_changed = message != *last_message;
    let tick_ready = last_emit_at.elapsed() >= Duration::from_millis(250);

    if force || progress_changed || message_changed || tick_ready {
        emit_backup_progress(
            app,
            phase,
            phase_progress,
            total_progress,
            speed_bps,
            frequency,
            backend,
            message.clone(),
        );
        *last_emit_at = Instant::now();
        *last_phase_progress = phase_progress;
        *last_message = message;
    }
}

pub(crate) fn classify_dump_progress(phase: &str, line: &str) -> Option<(f64, String)> {
    let normalized = line.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let subject = if phase == "mcu" { "MCU" } else { "SPI" };

    if normalized.contains("st-link") || normalized.contains("debug probe") || normalized.contains("programmer") {
        return Some((12.0, format!("{subject} probe detected")));
    }
    if normalized.contains("connect") || normalized.contains("attaching") || normalized.contains("swd") {
        return Some((22.0, format!("Connecting {subject} reader")));
    }
    if normalized.contains("target voltage") || normalized.contains("dap") || normalized.contains("halt") {
        return Some((34.0, format!("{subject} target linked")));
    }
    if normalized.contains("bank1") || normalized.contains("extflash") || normalized.contains("external flash") {
        return Some((46.0, format!("{subject} memory region selected")));
    }
    if normalized.contains("reading") || normalized.contains("dump") || normalized.contains("read memory") {
        return Some((58.0, format!("Reading {subject} data")));
    }
    if normalized.contains("writing") || normalized.contains("save") || normalized.contains("saved") {
        return Some((88.0, format!("Saving {subject} dump")));
    }

    None
}

pub(crate) fn parse_tqdm_spi_progress(line: &str) -> Option<(f64, f64, String)> {
    let percent_pos = line.find('%')?;
    let percent_start = line[..percent_pos]
        .rfind(|char: char| !char.is_ascii_digit())
        .map(|index| index + 1)
        .unwrap_or(0);
    let percent_text = line[percent_start..percent_pos].trim();
    let percent = percent_text.parse::<f64>().ok()?;

    let slash_pos = line.find('/')?;
    let current_start = line[..slash_pos]
        .rfind(|char: char| !char.is_ascii_digit())
        .map(|index| index + 1)
        .unwrap_or(0);
    let current_text = line[current_start..slash_pos].trim();
    let total_start = slash_pos + 1;
    let total_end = line[total_start..]
        .find(|char: char| !char.is_ascii_digit())
        .map(|index| total_start + index)
        .unwrap_or(line.len());
    let total_text = line[total_start..total_end].trim();

    let current = current_text.parse::<u64>().ok()?;
    let total = total_text.parse::<u64>().ok()?;
    if total == 0 {
        return None;
    }
    let computed_percent = ((current as f64 / total as f64) * 100.0).clamp(0.0, 100.0);
    let progress = percent.max(computed_percent);

    let trailing_metric = line
        .split(',')
        .last()
        .map(|item| item.trim().trim_end_matches(']'))
        .unwrap_or("");

    let speed_bps = trailing_metric
        .strip_suffix("it/s")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .map(|iters_per_second| iters_per_second * 256.0 * 1024.0)
        .or_else(|| {
            trailing_metric
                .strip_suffix("s/it")
                .and_then(|value| value.trim().parse::<f64>().ok())
                .filter(|seconds_per_iter| *seconds_per_iter > 0.0)
                .map(|seconds_per_iter| (256.0 * 1024.0) / seconds_per_iter)
        })
        .unwrap_or(0.0);

    let message = format!("Reading SPI chunk {current}/{total}");
    Some((progress, speed_bps.max(0.0), message))
}

fn run_backup_phase(
    _app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    model: String,
    _external_flash_mb: f64,
) -> Result<(&'static str, String, u32, PathBuf, PathBuf), String> {
    let model_name = resolve_model_name(&model)?;

    let backup_dir = required_active_backups_dir()?;
    fs::create_dir_all(&backup_dir).map_err(|error| format!("failed to create backups dir: {error}"))?;

    let internal_name = firmware_output_name("bank1", model_name);
    let external_name = firmware_output_name("spi", model_name);
    let internal_path = backup_dir.join(&internal_name);
    let external_path = backup_dir.join(&external_name);

    Ok((model_name, backend, frequency, internal_path, external_path))
}

fn run_single_backup_phase(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    model: String,
    protection: String,
    external_flash_mb: f64,
    phase: &str,
) -> Result<BackupReadResult, String> {
    let (model_name, initial_backend, _initial_frequency, internal_path, external_path) =
        run_backup_phase(app.clone(), backend, frequency, model, external_flash_mb)?;

    let bank2_path = internal_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(firmware_output_name("bank2", model_name));
    let (destination, dump_target, expected_size, total_base, total_span, output_label, name) = match phase {
        "mcu" => (
            internal_path,
            "bank1",
            0x40000_u64,
            0.0,
            100.0,
            "BANK1",
            firmware_output_name("bank1", model_name),
        ),
        "bank2" => (
            bank2_path,
            "bank2",
            0x40000_u64,
            0.0,
            100.0,
            "BANK2",
            firmware_output_name("bank2", model_name),
        ),
        _ => (
            external_path,
            "ext",
            ((external_flash_mb.max(1.0)) * 1024.0 * 1024.0) as u64,
            0.0,
            100.0,
            "SPI",
            firmware_output_name("spi", model_name),
        ),
    };

    if phase != "spi" && protection.trim().eq_ignore_ascii_case("LOCKED") {
        return Err(format!("{output_label} backup is unavailable while protection is locked"));
    }

    if phase == "spi" {
        let gnw_output = run_gnwmanager_spi_read(
            &app,
            frequency,
            &destination,
            expected_size,
        )?;
        if gnw_output.status.success() {
            mirror_backup_to_stock_name(&destination, "spi", model_name)?;
        }
        let stderr_text = String::from_utf8_lossy(&gnw_output.stderr).to_string();
        return Ok(BackupReadResult {
            summary: if gnw_output.status.success() {
                format!("SPI backup completed successfully (gnwmanager, freq {frequency})")
            } else {
                format!("GNWManager SPI read failed")
            },
            path: destination.to_string_lossy().to_string(),
            name,
            backend: "gnwmanager".to_string(),
            phase: phase.to_string(),
            frequency,
            speed_bps: 0.0,
            stderr: stderr_text,
        });
    }

    let used_backend = initial_backend;
    if used_backend.eq_ignore_ascii_case("pyocd") && (phase == "mcu" || phase == "bank2") {
        let direct_address = if phase == "mcu" { 0x0800_0000_u32 } else { 0x0810_0000_u32 };
        let mut direct_frequencies = Vec::new();
        for value in [1_000_000_u32, 500_000, 240_000, 100_000, frequency] {
            if value > 0 && !direct_frequencies.contains(&value) {
                direct_frequencies.push(value);
            }
        }
        let mut last_direct_error = String::new();
        for direct_frequency in direct_frequencies {
            match run_pyocd_internal_dump_under_reset(
                &app,
                phase,
                direct_address,
                &destination,
                expected_size,
                direct_frequency,
            ) {
                Ok(()) => {
                    let stock_part = if phase == "mcu" { "bank1" } else { "bank2" };
                    mirror_backup_to_stock_name(&destination, stock_part, model_name)?;
                    return Ok(BackupReadResult {
                        summary: format!("{output_label} backup completed successfully (pyocd direct under-reset, freq {direct_frequency})"),
                        path: destination.to_string_lossy().to_string(),
                        name,
                        backend: "pyocd".to_string(),
                        phase: phase.to_string(),
                        frequency: direct_frequency,
                        speed_bps: 0.0,
                        stderr: String::new(),
                    });
                }
                Err(error) => {
                    last_direct_error = error;
                    emit_backup_progress(
                        &app,
                        phase,
                        0.0,
                        0.0,
                        0.0,
                        direct_frequency,
                        "pyocd",
                        format!("Direct {output_label} read failed, trying fallback"),
                    );
                }
            }
        }
        emit_backup_progress(
            &app,
            phase,
            0.0,
            0.0,
            0.0,
            frequency,
            &used_backend,
            format!("Direct {output_label} read failed: {last_direct_error}"),
        );
    }
    let frequency_attempts = build_backend_frequency_attempts(&used_backend, frequency);
    let mut selected_frequency = frequency_attempts[0];
    let mut final_output = None;

    for candidate_frequency in &frequency_attempts {
        selected_frequency = *candidate_frequency;
        emit_backup_progress(
            &app,
            phase,
            0.0,
            0.0,
            0.0,
            selected_frequency,
            &used_backend,
            format!("Trying {output_label} backup at {selected_frequency}"),
        );
        let output = run_dump_with_progress(
            &app,
            &used_backend,
            selected_frequency,
            phase,
            &destination,
            expected_size,
            total_base,
            total_span,
            dump_target,
        )?;
        if output.status.success() {
            final_output = Some(output);
            break;
        }
    }

    let output = final_output.ok_or_else(|| format!("failed to dump {phase} backup after trying fallback frequencies"))?;
    let text = output_text(&output);
    if !output.status.success() {
        return Err(format!(
            "failed to dump {phase} backup: {}",
            text.lines().last().unwrap_or("unknown error")
        ));
    }
    let stock_part = if phase == "mcu" { "bank1" } else { "bank2" };
    mirror_backup_to_stock_name(&destination, stock_part, model_name)?;

    Ok(BackupReadResult {
        summary: format!("{output_label} backup completed successfully ({used_backend}, freq {selected_frequency})"),
        path: destination.to_string_lossy().to_string(),
        name,
        backend: used_backend,
        phase: phase.to_string(),
        frequency: selected_frequency,
        speed_bps: 0.0,
        stderr: String::new(),
    })
}

#[tauri::command]
pub(crate) async fn read_mcu_backup(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    model: String,
    protection: String,
    external_flash_mb: f64,
) -> Result<BackupReadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_backup_phase(app, backend, frequency, model, protection, external_flash_mb, "mcu")
    })
    .await
    .map_err(|error| format!("failed to join MCU backup task: {error}"))?
}

#[tauri::command]
pub(crate) async fn read_bank2_backup(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    model: String,
    protection: String,
    external_flash_mb: f64,
) -> Result<BackupReadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_backup_phase(app, backend, frequency, model, protection, external_flash_mb, "bank2")
    })
    .await
    .map_err(|error| format!("failed to join bank2 backup task: {error}"))?
}

#[tauri::command]
pub(crate) async fn read_spi_backup(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    model: String,
    protection: String,
    external_flash_mb: f64,
) -> Result<BackupReadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_backup_phase(app, backend, frequency, model, protection, external_flash_mb, "spi")
    })
    .await
    .map_err(|error| format!("failed to join SPI backup task: {error}"))?
}
