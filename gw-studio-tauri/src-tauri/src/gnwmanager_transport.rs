use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::backup_read::{
    classify_dump_progress, emit_backup_debug, emit_backup_progress_throttled,
    parse_tqdm_spi_progress,
};
use crate::device::gnwmanager_argv;
use crate::process_stream::forward_stream_updates;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn hide_command_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub(crate) fn run_dump_with_progress(
    app: &tauri::AppHandle,
    backend: &str,
    frequency: u32,
    phase: &str,
    destination: &PathBuf,
    expected_size: u64,
    total_base: f64,
    total_span: f64,
    dump_target: &str,
) -> Result<Output, String> {
    if destination.exists() {
        let _ = fs::remove_file(destination);
    }

    let argv = gnwmanager_argv();
    let mut command = if argv.len() == 1 {
        Command::new(&argv[0])
    } else {
        let mut cmd = Command::new(&argv[0]);
        for part in argv.iter().skip(1) {
            cmd.arg(part);
        }
        cmd
    };
    command
        .arg("-b")
        .arg(backend)
        .arg("-f")
        .arg(frequency.to_string())
        .arg("dump")
        .arg(dump_target)
        .arg(destination.to_string_lossy().to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = hide_command_window(&mut command).spawn()
        .map_err(|error| format!("failed to spawn dump process: {error}"))?;

    let started_at = Instant::now();
    let mut last_size = 0u64;
    let mut last_tick = Instant::now();
    let mut last_emit_at = Instant::now() - Duration::from_secs(1);
    let mut last_emitted_phase_progress = -1.0_f64;
    let mut last_emitted_message = String::new();
    let mut staged_progress = if phase == "spi" { 0.0_f64 } else { 4.0_f64 };
    let mut tqdm_progress: Option<f64> = None;
    let mut tqdm_speed_bps: Option<f64> = None;

    emit_backup_progress_throttled(
        app,
        phase,
        staged_progress,
        (total_base + (staged_progress / 100.0) * total_span).clamp(0.0, 100.0),
        0.0,
        frequency,
        backend,
        format!("Starting {phase} dump"),
        &mut last_emit_at,
        &mut last_emitted_phase_progress,
        &mut last_emitted_message,
        true,
    );

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture dump stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture dump stderr".to_string())?;

    let (tx, rx) = mpsc::channel::<(bool, String)>();

    let stdout_handle = forward_stream_updates(stdout, tx.clone(), true);
    let stderr_handle = forward_stream_updates(stderr, tx.clone(), false);

    drop(tx);
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();

    let status = loop {
        while let Ok((is_stdout, line)) = rx.try_recv() {
            if is_stdout {
                stdout_text.push_str(&line);
                stdout_text.push('\n');
            } else {
                stderr_text.push_str(&line);
                stderr_text.push('\n');
            }

            if phase == "spi" {
                emit_backup_debug(app, phase, if is_stdout { "stdout" } else { "stderr" }, &line);
            }

            if let Some((progress, message)) = classify_dump_progress(phase, &line) {
                staged_progress = staged_progress.max(progress);
                let stage_progress = if phase == "spi" {
                    tqdm_progress.unwrap_or(0.0)
                } else {
                    staged_progress
                };
                emit_backup_progress_throttled(
                    app,
                    phase,
                    stage_progress,
                    (total_base + (stage_progress / 100.0) * total_span).clamp(0.0, 100.0),
                    0.0,
                    frequency,
                    backend,
                    message,
                    &mut last_emit_at,
                    &mut last_emitted_phase_progress,
                    &mut last_emitted_message,
                    false,
                );
            }

            if phase == "spi" {
                if let Some((progress, speed_bps, message)) = parse_tqdm_spi_progress(&line) {
                    tqdm_progress = Some(progress);
                    tqdm_speed_bps = Some(speed_bps);
                    emit_backup_progress_throttled(
                        app,
                        phase,
                        progress,
                        (total_base + (progress / 100.0) * total_span).clamp(0.0, 100.0),
                        speed_bps,
                        frequency,
                        backend,
                        message,
                        &mut last_emit_at,
                        &mut last_emitted_phase_progress,
                        &mut last_emitted_message,
                        false,
                    );
                }
            }
        }

        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll dump process: {error}"))?
        {
            break status;
        }

        let current_size = fs::metadata(destination).map(|meta| meta.len()).unwrap_or(0);
        let elapsed = last_tick.elapsed().as_secs_f64().max(0.001);
        let delta = current_size.saturating_sub(last_size) as f64;
        let speed_bps = delta / elapsed;
        last_size = current_size;
        last_tick = Instant::now();

        let measured_progress = if expected_size == 0 {
            0.0
        } else {
            ((current_size as f64 / expected_size as f64) * 100.0).clamp(0.0, 100.0)
        };
        let phase_progress = if phase == "spi" {
            if let Some(progress) = tqdm_progress {
                progress
            } else if measured_progress > 0.0 {
                measured_progress
            } else {
                0.0
            }
        } else {
            measured_progress.max(staged_progress)
        };
        let reported_speed_bps = tqdm_speed_bps.unwrap_or(speed_bps);
        let total_progress = (total_base + (phase_progress / 100.0) * total_span).clamp(0.0, 100.0);

        emit_backup_progress_throttled(
            app,
            phase,
            phase_progress,
            total_progress,
            reported_speed_bps,
            frequency,
            backend,
            if current_size > 0 {
                format!("{phase} dump in progress")
            } else {
                format!("{phase} probe active")
            },
            &mut last_emit_at,
            &mut last_emitted_phase_progress,
            &mut last_emitted_message,
            false,
        );

        thread::sleep(Duration::from_millis(180));
    };

    while let Ok((is_stdout, line)) = rx.try_recv() {
        if is_stdout {
            stdout_text.push_str(&line);
            stdout_text.push('\n');
        } else {
            stderr_text.push_str(&line);
            stderr_text.push('\n');
        }
    }

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    if status.success() {
        let current_size = fs::metadata(destination).map(|meta| meta.len()).unwrap_or(0);
        let speed_bps = (current_size as f64 / started_at.elapsed().as_secs_f64().max(0.001)).max(0.0);
        emit_backup_progress_throttled(
            app,
            phase,
            100.0,
            (total_base + total_span).clamp(0.0, 100.0),
            speed_bps,
            frequency,
            backend,
            format!("{phase} dump finished"),
            &mut last_emit_at,
            &mut last_emitted_phase_progress,
            &mut last_emitted_message,
            true,
        );
    }

    Ok(Output {
        status,
        stdout: stdout_text.into_bytes(),
        stderr: stderr_text.into_bytes(),
    })
}

pub(crate) fn run_flash_command(
    backend: &str,
    frequency: u32,
    target: &str,
    source: &PathBuf,
) -> Result<Output, String> {
    let argv = gnwmanager_argv();
    let mut command = if argv.len() == 1 {
        Command::new(&argv[0])
    } else {
        let mut cmd = Command::new(&argv[0]);
        for part in argv.iter().skip(1) {
            cmd.arg(part);
        }
        cmd
    };
    command
        .arg("-b")
        .arg(backend)
        .arg("-f")
        .arg(frequency.to_string())
        .arg("flash")
        .arg(target)
        .arg(source.to_string_lossy().to_string());

    hide_command_window(&mut command)
        .output()
        .map_err(|error| format!("failed to run gnwmanager flash {target}: {error}"))
}
