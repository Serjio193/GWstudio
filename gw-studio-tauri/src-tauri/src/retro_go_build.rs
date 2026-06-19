use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::build_events::{emit_build_progress, read_lossy_output_line};
use crate::toolchain::{
    build_job_count, ensure_msys_tmp_dir, locate_gcc_bin_dir, locate_git_bin_dir, locate_make_exe,
    to_bash_path_for,
};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn hide_command_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub(crate) fn run_retro_go_build(
    app: &tauri::AppHandle,
    workspace_dir: &PathBuf,
    extflash_size_mb: u32,
    extflash_offset_bytes: u64,
    intflash_bank: u8,
    firmware_code: &str,
    coverflow_enabled: bool,
) -> Result<PathBuf, String> {
    let make_exe = locate_make_exe().ok_or_else(|| "mingw32-make.exe not found".to_string())?;
    let gcc_bin = locate_gcc_bin_dir().ok_or_else(|| "arm-none-eabi toolchain not found".to_string())?;
    let git_bin = locate_git_bin_dir().ok_or_else(|| "Git bash/sh not found".to_string())?;
    let build_log_path = workspace_dir.join("build_gw_studio.log");
    let target = if firmware_code == "z" { "zelda" } else { "mario" };
    let bash_exe = git_bin.join("bash.exe");
    let coverflow_flag = if coverflow_enabled { "1" } else { "0" };
    let make_dir = make_exe
        .parent()
        .ok_or_else(|| "failed to resolve mingw32-make.exe dir".to_string())?;
    let jobs = build_job_count();
    ensure_msys_tmp_dir(&git_bin)?;
    let bash_command = format!(
        "export PATH=\"{}:{}:{}:$PATH\"; cd \"{}\"; \"{}\" SHELL=\"{}\" -j{} VERBOSE=1 CHECK_TOOLS=0 CHECK_DIRTY_SUBMODULE=0 COVERFLOW={} JPG_QUALITY=90 GNW_TARGET={} EXTFLASH_SIZE_MB={} EXTFLASH_OFFSET={} INTFLASH_BANK={} build/gw_retro_go_intflash.bin build/gw_retro_go_extflash.bin 2>&1",
        to_bash_path_for(&git_bin, &git_bin),
        to_bash_path_for(make_dir, &git_bin),
        to_bash_path_for(&gcc_bin, &git_bin),
        to_bash_path_for(workspace_dir, &git_bin),
        to_bash_path_for(&make_exe, &git_bin),
        to_bash_path_for(&bash_exe, &git_bin),
        jobs,
        coverflow_flag,
        target,
        extflash_size_mb,
        extflash_offset_bytes,
        intflash_bank
    );
    emit_build_progress(app, 68.0, "Starting Retro-Go fork build");

    let mut command = Command::new(&bash_exe);
    command
        .current_dir(workspace_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .arg("-lc")
        .arg(&bash_command);
    let mut child = hide_command_window(&mut command)
        .spawn()
        .map_err(|error| format!("failed to launch Retro-Go fork make: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture Retro-Go fork build output".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut combined = String::new();
    let mut compile_progress = 68.0_f64;
    let mut last_emitted = -1_i32;

    loop {
        let line = read_lossy_output_line(&mut reader)
            .map_err(|error| format!("failed reading Retro-Go fork build output: {error}"))?;
        let Some(line) = line else {
            break;
        };
        combined.push_str(&line);
        combined.push('\n');

        if line.contains("[ WGET ]") {
            compile_progress = (compile_progress + 0.12).min(74.0);
        } else if line.contains("[ PYTHON3 ]") || line.contains("[ BASH ]") {
            compile_progress = compile_progress.max(75.0);
        } else if line.contains("[ CC ") {
            compile_progress = (compile_progress + 0.05).min(96.0);
        } else if line.contains("[ LD ]") {
            compile_progress = 97.0;
        } else if line.contains("External flash usage") {
            compile_progress = 98.0;
        } else if line.contains("[ BIN ]") {
            compile_progress = 99.0;
        }

        let rounded = compile_progress.round() as i32;
        if rounded != last_emitted {
            last_emitted = rounded;
            emit_build_progress(app, compile_progress, line.clone());
        }
    }

    let status = child
        .wait()
        .map_err(|error| format!("failed to wait for Retro-Go fork build: {error}"))?;
    fs::write(&build_log_path, &combined)
        .map_err(|error| format!("failed to write build log: {error}"))?;

    if !status.success() {
        if let Some(overflow_line) = combined
            .lines()
            .find(|line| line.contains("region `EXTFLASH' overflowed by"))
        {
            return Err(format!(
                "Retro-Go fork build failed: образ не помещается в SPI. {overflow_line}. Уберите несколько игр или обложек и повторите сборку. Log: {}",
                build_log_path.to_string_lossy()
            ));
        }
        return Err(format!(
            "Retro-Go fork build failed; see {}",
            build_log_path.to_string_lossy()
        ));
    }

    emit_build_progress(app, 100.0, "Extflash build complete");
    Ok(build_log_path)
}
