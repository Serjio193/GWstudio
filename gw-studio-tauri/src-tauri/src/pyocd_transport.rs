use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::backup_read::emit_backup_progress;
use crate::flash::emit_firmware_write_progress;
use crate::process_stream::forward_stream_updates;
use crate::toolchain::locate_python_exe;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn hide_command_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub(crate) fn run_pyocd_internal_dump_under_reset(
    app: &tauri::AppHandle,
    phase: &str,
    address: u32,
    destination: &Path,
    expected_size: u64,
    frequency: u32,
) -> Result<(), String> {
    if destination.exists() {
        let _ = fs::remove_file(destination);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create backup output dir: {error}"))?;
    }

    emit_backup_progress(
        app,
        phase,
        0.0,
        0.0,
        0.0,
        frequency,
        "pyocd",
        format!("Starting {phase} direct read under reset"),
    );

    let script = r#"
import sys
from pathlib import Path
from gnwmanager.ocdbackend.pyocd_backend import PyOCDBackend

address = int(sys.argv[1], 0)
size = int(sys.argv[2], 0)
frequency = int(sys.argv[3], 0)
output = Path(sys.argv[4])
chunk_size = 8192

backend = PyOCDBackend(connect_mode="under-reset")
try:
    backend.open()
    backend.set_frequency(frequency)
    done = 0
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as handle:
        while done < size:
            chunk = min(chunk_size, size - done)
            data = backend.read_memory(address + done, chunk)
            handle.write(data)
            done += len(data)
            print(f"progress={done}/{size}", flush=True)
finally:
    try:
        backend.close()
    except Exception:
        pass
"#;

    let python = locate_python_exe();
    let mut command = Command::new(&python);
    command
        .arg("-u")
        .arg("-c")
        .arg(script)
        .arg(format!("0x{address:08X}"))
        .arg(expected_size.to_string())
        .arg(frequency.to_string())
        .arg(destination.to_string_lossy().to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = hide_command_window(&mut command)
        .spawn()
        .map_err(|error| format!("failed to spawn pyocd direct dump: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture pyocd direct dump stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture pyocd direct dump stderr".to_string())?;
    let (tx, rx) = mpsc::channel::<(bool, String)>();
    let stdout_handle = forward_stream_updates(stdout, tx.clone(), true);
    let stderr_handle = forward_stream_updates(stderr, tx.clone(), false);
    drop(tx);

    let started_at = Instant::now();
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();

    loop {
        while let Ok((is_stdout, line)) = rx.try_recv() {
            if is_stdout {
                stdout_text.push_str(&line);
                stdout_text.push('\n');
                if let Some(value) = line.strip_prefix("progress=") {
                    if let Some((done_text, total_text)) = value.split_once('/') {
                        let done = done_text.trim().parse::<u64>().unwrap_or(0);
                        let total = total_text.trim().parse::<u64>().unwrap_or(expected_size);
                        let percent = if total > 0 {
                            ((done as f64 / total as f64) * 100.0).clamp(0.0, 100.0)
                        } else {
                            0.0
                        };
                        let speed_bps = done as f64 / started_at.elapsed().as_secs_f64().max(0.001);
                        emit_backup_progress(
                            app,
                            phase,
                            percent,
                            percent,
                            speed_bps,
                            frequency,
                            "pyocd",
                            format!("{phase} direct read under reset"),
                        );
                    }
                }
            } else {
                stderr_text.push_str(&line);
                stderr_text.push('\n');
            }
        }

        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll pyocd direct dump: {error}"))?
        {
            let _ = stdout_handle.join();
            let _ = stderr_handle.join();
            if status.success() {
                let size = fs::metadata(destination).map(|metadata| metadata.len()).unwrap_or(0);
                if size != expected_size {
                    return Err(format!(
                        "pyocd direct dump size mismatch: got {size} bytes, expected {expected_size}"
                    ));
                }
                let speed_bps = size as f64 / started_at.elapsed().as_secs_f64().max(0.001);
                emit_backup_progress(
                    app,
                    phase,
                    100.0,
                    100.0,
                    speed_bps,
                    frequency,
                    "pyocd",
                    format!("{phase} direct read finished"),
                );
                return Ok(());
            }
            let mut text = stdout_text;
            if !stderr_text.trim().is_empty() {
                text.push('\n');
                text.push_str(&stderr_text);
            }
            return Err(format!(
                "pyocd direct dump failed: {}",
                text.lines().last().unwrap_or("unknown error")
            ));
        }

        thread::sleep(Duration::from_millis(80));
    }
}

pub(crate) fn run_pyocd_internal_flash_under_reset(
    app: &tauri::AppHandle,
    target: &str,
    bank: u8,
    source: &Path,
    frequency: u32,
) -> Result<Output, String> {
    emit_firmware_write_progress(
        app,
        target,
        "write",
        0.0,
        0.0,
        frequency,
        "pyocd",
        format!("Starting {target} flash under reset"),
    );

    let script = r#"
import sys
from pathlib import Path
from gnwmanager.gnw import GnW
from gnwmanager.ocdbackend.pyocd_backend import PyOCDBackend

bank = int(sys.argv[1], 0)
frequency = int(sys.argv[2], 0)
source = Path(sys.argv[3])
data = source.read_bytes()

backend = PyOCDBackend(connect_mode="under-reset")
try:
    print("progress=5 preparing", flush=True)
    backend.open()
    backend.set_frequency(frequency)
    gnw = GnW(backend)
    print("progress=15 starting_gnwmanager", flush=True)
    gnw.start_gnwmanager()
    print("progress=35 flashing", flush=True)
    gnw.flash(bank, 0, data, progress=False)
    print("progress=100 done", flush=True)
finally:
    try:
        backend.close()
    except Exception:
        pass
"#;

    let python = locate_python_exe();
    let mut command = Command::new(&python);
    command
        .arg("-u")
        .arg("-c")
        .arg(script)
        .arg(bank.to_string())
        .arg(frequency.to_string())
        .arg(source.to_string_lossy().to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = hide_command_window(&mut command)
        .spawn()
        .map_err(|error| format!("failed to spawn pyocd internal flash: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture pyocd internal flash stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture pyocd internal flash stderr".to_string())?;
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
                if let Some(rest) = line.strip_prefix("progress=") {
                    let mut parts = rest.splitn(2, ' ');
                    let percent = parts
                        .next()
                        .and_then(|value| value.trim().parse::<f64>().ok())
                        .unwrap_or(0.0)
                        .clamp(0.0, 100.0);
                    let message = parts.next().unwrap_or("flashing").replace('_', " ");
                    emit_firmware_write_progress(
                        app,
                        target,
                        "write",
                        percent,
                        percent,
                        frequency,
                        "pyocd",
                        message,
                    );
                }
            } else {
                stderr_text.push_str(&line);
                stderr_text.push('\n');
            }
        }

        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll pyocd internal flash: {error}"))?
        {
            break status;
        }

        thread::sleep(Duration::from_millis(100));
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

    Ok(Output {
        status,
        stdout: stdout_text.into_bytes(),
        stderr: stderr_text.into_bytes(),
    })
}
