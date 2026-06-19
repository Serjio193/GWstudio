use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::backup_read::{emit_backup_debug, emit_backup_progress};
use crate::flash::emit_firmware_write_progress;
use crate::paths::runtime_tools_dir;
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

fn gnwmanager_python_command() -> Command {
    let python = locate_python_exe();
    if python.exists() {
        return Command::new(python);
    }
    let mut command = Command::new("py");
    command.arg("-3");
    command
}

fn write_gnwmanager_spi_helper() -> Result<PathBuf, String> {
    let script_dir = runtime_tools_dir();
    fs::create_dir_all(&script_dir)
        .map_err(|error| format!("failed to create runtime tools dir: {error}"))?;
    let script_path = script_dir.join("gnw_spi_progress.py");
    let script = r#"from __future__ import annotations

import argparse
import sys
import time
from collections import namedtuple
from pathlib import Path

from gnwmanager.gnw import GnW, chunk_bytes, pad_bytes, sha256
from gnwmanager.ocdbackend.pyocd_backend import PyOCDBackend


def open_gnw(frequency: int) -> GnW:
    backend = PyOCDBackend(connect_mode="under-reset")
    backend.open()
    backend.set_frequency(frequency)
    gnw = GnW(backend)
    gnw.start_gnwmanager()
    return gnw


def emit(kind: str, progress: int, done: int, total: int, started: float) -> None:
    elapsed = max(time.perf_counter() - started, 0.001)
    speed = done / elapsed
    print(f"GNW_{kind}_PROGRESS {progress} {done} {total} {speed:.0f}", flush=True)


def read_ext(args: argparse.Namespace) -> None:
    gnw = open_gnw(args.frequency)
    total = args.size or int(gnw.external_flash_size)
    chunk = max(4096, args.chunk)
    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        out.unlink()
    started = time.perf_counter()
    done = 0
    emit("READ", 0, 0, total, started)
    with out.open("wb") as handle:
        while done < total:
            size = min(chunk, total - done)
            handle.write(gnw.read_memory(0x90000000 + done, size))
            done += size
            emit("READ", int(done * 100 / total), done, total, started)
    print(f"GNW_READ_DONE {time.perf_counter() - started:.3f}", flush=True)


def erase_ext(args: argparse.Namespace) -> None:
    gnw = open_gnw(args.frequency)
    total = args.size or int(gnw.external_flash_size)
    block = int(gnw.external_flash_block_size) or 4096
    chunk = max(block, args.chunk)
    chunk = max(block, (chunk // block) * block)
    started = time.perf_counter()
    done = 0
    print(f"GNW_ERASE_BEGIN {total} {block} {chunk}", flush=True)
    emit("ERASE", 0, 0, total, started)
    while done < total:
        size = min(chunk, total - done)
        if size > block:
            size = (size // block) * block
        if size <= 0:
            size = total - done
        gnw.erase(0, done, size, whole_chip=False, timeout=10000)
        done += size
        emit("ERASE", int(done * 100 / total), done, total, started)
    print(f"GNW_ERASE_DONE {time.perf_counter() - started:.3f}", flush=True)


def verify_ext(gnw: GnW, offset: int, chunks: list[bytes], chunk_size: int, started: float) -> None:
    total_bytes = sum(len(chunk) for chunk in chunks)
    done = 0
    emit("VERIFY", 0, 0, total_bytes, started)
    group_size = max(1, (2 << 20) // chunk_size)
    for start in range(0, len(chunks), group_size):
        group = chunks[start:start + group_size]
        expected = [sha256(chunk) for chunk in group]
        size = sum(len(chunk) for chunk in group)
        actual = gnw.read_hashes(offset + start * chunk_size, size)
        if actual[:len(expected)] != expected:
            raise RuntimeError(f"verify failed at external flash offset 0x{offset + start * chunk_size:08X}")
        done += size
        emit("VERIFY", int(done * 100 / total_bytes), done, total_bytes, started)
    print(f"GNW_VERIFY_DONE {time.perf_counter() - started:.3f}", flush=True)


def write_ext(args: argparse.Namespace) -> None:
    gnw = open_gnw(args.frequency)
    source = Path(args.input)
    data = pad_bytes(source.read_bytes(), int(gnw.external_flash_block_size))
    if len(data) > int(gnw.external_flash_size):
        raise ValueError("input does not fit into external flash")

    started = time.perf_counter()
    print(f"GNW_WRITE_HASH_START {len(data)}", flush=True)
    hash_started = time.perf_counter()
    device_hashes = gnw.read_hashes(args.offset, len(data))
    print(f"GNW_WRITE_HASH_DONE {time.perf_counter() - hash_started:.3f}", flush=True)

    chunk_size = gnw.contexts[0]["buffer"].size
    chunks = chunk_bytes(data, chunk_size)
    Packet = namedtuple("Packet", ["addr", "data"])
    all_packets = [Packet(args.offset + i * chunk_size, chunk) for i, chunk in enumerate(chunks)]
    packets = [
        packet
        for packet, device_hash in zip(all_packets, device_hashes)
        if sha256(packet.data) != device_hash
    ]
    total = len(packets)
    print(f"GNW_WRITE_PACKETS {total} {len(all_packets)} {chunk_size}", flush=True)
    if total == 0:
        print(f"GNW_WRITE_DONE {time.perf_counter() - started:.3f} skipped", flush=True)
        verify_ext(gnw, args.offset, chunks, chunk_size, started)
        return

    emit("WRITE", 0, 0, total, started)
    written = 0
    total_bytes = sum(len(packet.data) for packet in packets)
    for idx, packet in enumerate(packets, start=1):
        gnw.program(0, packet.addr, packet.data, blocking=False)
        gnw.write_uint32("progress", int(26 * idx / total))
        gnw.wait_for_all_contexts_complete()
        written += len(packet.data)
        emit("WRITE", int(idx * 100 / total), written, total_bytes, started)
    print(f"GNW_WRITE_DONE {time.perf_counter() - started:.3f}", flush=True)
    verify_ext(gnw, args.offset, chunks, chunk_size, started)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--frequency", type=int, default=8_000_000)
    sub = parser.add_subparsers(dest="command", required=True)

    read = sub.add_parser("read-ext")
    read.add_argument("--output", required=True)
    read.add_argument("--size", type=int, default=0)
    read.add_argument("--chunk", type=int, default=2 << 20)
    read.set_defaults(func=read_ext)

    erase = sub.add_parser("erase-ext")
    erase.add_argument("--size", type=int, default=0)
    erase.add_argument("--chunk", type=int, default=2 << 20)
    erase.set_defaults(func=erase_ext)

    write = sub.add_parser("write-ext")
    write.add_argument("--input", required=True)
    write.add_argument("--offset", type=int, default=0)
    write.set_defaults(func=write_ext)

    args = parser.parse_args()
    args.func(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
"#;
    fs::write(&script_path, script)
        .map_err(|error| format!("failed to write GNWManager SPI helper: {error}"))?;
    Ok(script_path)
}

pub(crate) fn run_gnwmanager_spi_erase_chunks(
    app: &tauri::AppHandle,
    frequency: u32,
    expected_size: u64,
) -> Result<(), String> {
    let script_path = write_gnwmanager_spi_helper()?;

    let mut command = gnwmanager_python_command();
    command
        .arg("-u")
        .arg(&script_path)
        .arg("--frequency")
        .arg(frequency.to_string())
        .arg("erase-ext")
        .arg("--chunk")
        .arg((2 * 1024 * 1024).to_string())
        .arg("--size")
        .arg(expected_size.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = hide_command_window(&mut command)
        .spawn()
        .map_err(|error| format!("failed to spawn GNWManager SPI erase helper: {error}"))?;

    run_gnwmanager_spi_progress_child(
        app,
        child,
        "erase",
        frequency,
        "GNWManager SPI erase failed",
    )
    .map(|_| ())
}

pub(crate) fn run_gnwmanager_spi_read(
    app: &tauri::AppHandle,
    frequency: u32,
    destination: &PathBuf,
    expected_size: u64,
) -> Result<Output, String> {
    let script_path = write_gnwmanager_spi_helper()?;
    if destination.exists() {
        let _ = fs::remove_file(destination);
    }

    let mut command = gnwmanager_python_command();
    command
        .arg("-u")
        .arg(&script_path)
        .arg("--frequency")
        .arg(frequency.to_string())
        .arg("read-ext")
        .arg("--chunk")
        .arg((2 * 1024 * 1024).to_string())
        .arg("--size")
        .arg(expected_size.to_string())
        .arg("--output")
        .arg(destination)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = hide_command_window(&mut command)
        .spawn()
        .map_err(|error| format!("failed to spawn GNWManager SPI read helper: {error}"))?;

    run_gnwmanager_spi_progress_child(
        app,
        child,
        "read",
        frequency,
        "GNWManager SPI read failed",
    )
}

pub(crate) fn run_gnwmanager_spi_write(
    app: &tauri::AppHandle,
    frequency: u32,
    source: &PathBuf,
    offset_bytes: u64,
) -> Result<Output, String> {
    let script_path = write_gnwmanager_spi_helper()?;

    let mut command = gnwmanager_python_command();
    command
        .arg("-u")
        .arg(&script_path)
        .arg("--frequency")
        .arg(frequency.to_string())
        .arg("write-ext")
        .arg("--input")
        .arg(source)
        .arg("--offset")
        .arg(offset_bytes.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = hide_command_window(&mut command)
        .spawn()
        .map_err(|error| format!("failed to spawn GNWManager SPI write helper: {error}"))?;

    run_gnwmanager_spi_progress_child(
        app,
        child,
        "write",
        frequency,
        "GNWManager SPI write failed",
    )
}

fn handle_gnwmanager_spi_progress_line(
    app: &tauri::AppHandle,
    operation: &str,
    frequency: u32,
    is_read: bool,
    line: &str,
) -> bool {
    let progress_prefix = match operation {
        "read" => "GNW_READ_PROGRESS ",
        "erase" => "GNW_ERASE_PROGRESS ",
        "write" => "GNW_WRITE_PROGRESS ",
        _ => "",
    };
    if let Some(rest) = line.strip_prefix(progress_prefix) {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if let Some(percent_text) = parts.first() {
            if let Ok(percent) = percent_text.parse::<f64>() {
                let speed_bps = parts
                    .get(3)
                    .and_then(|value| value.parse::<f64>().ok())
                    .unwrap_or(0.0);
                if is_read {
                    emit_backup_progress(
                        app,
                        "spi",
                        percent,
                        percent,
                        speed_bps,
                        frequency,
                        "gnwmanager",
                        "Reading SPI flash",
                    );
                } else {
                    emit_firmware_write_progress(
                        app,
                        "spi",
                        operation,
                        percent,
                        speed_bps,
                        frequency,
                        "gnwmanager",
                        match operation {
                            "erase" => "Erasing SPI flash",
                            "write" => "Writing SPI flash",
                            _ => "Preparing SPI flash",
                        },
                    );
                }
            }
        }
        return false;
    }

    if let Some(rest) = line.strip_prefix("GNW_VERIFY_PROGRESS ") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if let Some(percent_text) = parts.first() {
            if let Ok(percent) = percent_text.parse::<f64>() {
                let speed_bps = parts
                    .get(3)
                    .and_then(|value| value.parse::<f64>().ok())
                    .unwrap_or(0.0);
                emit_firmware_write_progress(
                    app,
                    "spi",
                    "verify",
                    percent,
                    speed_bps,
                    frequency,
                    "gnwmanager",
                    "Verifying SPI flash",
                );
            }
        }
        return false;
    }

    if line.starts_with("GNW_WRITE_HASH_START ") {
        emit_firmware_write_progress(
            app,
            "spi",
            "prepare",
            0.0,
            0.0,
            frequency,
            "gnwmanager",
            "Checking SPI flash chunks",
        );
    } else if line.starts_with("GNW_WRITE_HASH_DONE ") {
        emit_firmware_write_progress(
            app,
            "spi",
            "prepare",
            100.0,
            0.0,
            frequency,
            "gnwmanager",
            "SPI chunk check finished",
        );
    } else if line.starts_with("GNW_WRITE_DONE ") && operation == "write" {
        emit_firmware_write_progress(
            app,
            "spi",
            "verify",
            0.0,
            0.0,
            frequency,
            "gnwmanager",
            "Verifying SPI flash",
        );
    } else if line.starts_with("GNW_VERIFY_DONE ") && operation == "write" {
        emit_firmware_write_progress(
            app,
            "spi",
            "verify",
            100.0,
            0.0,
            frequency,
            "gnwmanager",
            "SPI verify finished",
        );
        return true;
    }

    false
}

fn run_gnwmanager_spi_progress_child(
    app: &tauri::AppHandle,
    mut child: std::process::Child,
    operation: &str,
    frequency: u32,
    error_prefix: &str,
) -> Result<Output, String> {

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture GNWManager SPI helper stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture GNWManager SPI helper stderr".to_string())?;

    let (tx, rx) = mpsc::channel::<(bool, String)>();
    let stdout_handle = spawn_byte_reader(stdout, tx.clone());
    let stderr_handle = spawn_byte_reader(stderr, tx.clone());
    drop(tx);

    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut pending_line = String::new();
    let mut verify_done = false;

    let is_read = operation == "read";
    if is_read {
        emit_backup_progress(
            app,
            "spi",
            0.0,
            0.0,
            0.0,
            frequency,
            "gnwmanager",
            "Reading SPI flash",
        );
    } else {
        emit_firmware_write_progress(
            app,
            "spi",
            operation,
            0.0,
            0.0,
            frequency,
            "gnwmanager",
            match operation {
                "erase" => "Erasing SPI flash",
                "write" => "Writing SPI flash",
                _ => "Preparing SPI flash",
            },
        );
    }

    let status = loop {
        while let Ok((is_stdout, text)) = rx.try_recv() {
            if is_stdout {
                stdout_text.push_str(&text);
                pending_line.push_str(&text);
                while let Some(index) = pending_line.find('\n') {
                    let line = pending_line[..index].trim().to_string();
                    pending_line = pending_line[index + 1..].to_string();
                    if handle_gnwmanager_spi_progress_line(app, operation, frequency, is_read, &line) {
                        verify_done = true;
                    }
                }
            } else {
                stderr_text.push_str(&text);
                if is_read {
                    emit_backup_debug(app, "spi", "stderr", &text);
                }
            }
        }

        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll GNWManager SPI helper: {error}"))?
        {
            break status;
        }

        thread::sleep(Duration::from_millis(100));
    };

    while let Ok((is_stdout, text)) = rx.try_recv() {
        if is_stdout {
            stdout_text.push_str(&text);
            pending_line.push_str(&text);
            while let Some(index) = pending_line.find('\n') {
                let line = pending_line[..index].trim().to_string();
                pending_line = pending_line[index + 1..].to_string();
                if handle_gnwmanager_spi_progress_line(app, operation, frequency, is_read, &line) {
                    verify_done = true;
                }
            }
        } else {
            stderr_text.push_str(&text);
        }
    }

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    while let Ok((is_stdout, text)) = rx.try_recv() {
        if is_stdout {
            stdout_text.push_str(&text);
            pending_line.push_str(&text);
            while let Some(index) = pending_line.find('\n') {
                let line = pending_line[..index].trim().to_string();
                pending_line = pending_line[index + 1..].to_string();
                if handle_gnwmanager_spi_progress_line(app, operation, frequency, is_read, &line) {
                    verify_done = true;
                }
            }
        } else {
            stderr_text.push_str(&text);
        }
    }

    if !pending_line.trim().is_empty() {
        let line = pending_line.trim().to_string();
        if handle_gnwmanager_spi_progress_line(app, operation, frequency, is_read, &line) {
            verify_done = true;
        }
    }

    if status.success() {
        if operation == "write" && !verify_done {
            return Err("GNWManager SPI write ended without verify completion marker".to_string());
        }
        if is_read {
            emit_backup_progress(
                app,
                "spi",
                100.0,
                100.0,
                0.0,
                frequency,
                "gnwmanager",
                "SPI read finished",
            );
        } else {
            let final_stage = if operation == "write" { "verify" } else { operation };
            emit_firmware_write_progress(
                app,
                "spi",
                final_stage,
                100.0,
                0.0,
                frequency,
                "gnwmanager",
                match operation {
                    "erase" => "SPI erase finished",
                    "write" => "SPI verify finished",
                    _ => "SPI operation finished",
                },
            );
        }
        Ok(Output {
            status,
            stdout: stdout_text.into_bytes(),
            stderr: stderr_text.into_bytes(),
        })
    } else {
        let text = if !stderr_text.trim().is_empty() {
            stderr_text
        } else {
            stdout_text
        };
        Err(format!(
            "{}: {}",
            error_prefix,
            text.lines().last().unwrap_or("unknown error")
        ))
    }
}

fn spawn_byte_reader<R: Read + Send + 'static>(
    reader: R,
    sender: mpsc::Sender<(bool, String)>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0_u8; 256];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = sender.send((true, s));
                }
                Err(_) => break,
            }
        }
    })
}
