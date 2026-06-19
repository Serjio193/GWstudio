use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::build_events::{emit_build_progress, read_lossy_output_line};
use crate::build_workspace::{copy_dir_filtered, ensure_clean_dir};
use crate::firmware_image::copy_file_prefix;
use crate::paths::{host_root, portable_source_dir};
use crate::stock::validate_patch_stock_inputs;
use crate::toolchain::{
    build_job_count, ensure_msys_tmp_dir, locate_gcc_bin_dir, locate_git_bin_dir, locate_make_exe,
    locate_python_exe, to_bash_path_for,
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

fn locate_game_watch_patch_repo() -> Option<PathBuf> {
    let candidates = [
        host_root().join("game-and-watch-patch"),
        host_root().join("game-and-watch-patch-clean"),
        portable_source_dir().join("game-and-watch-patch"),
        portable_source_dir().join("game-and-watch-patch-clean"),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("patch.py").exists() && path.join("Makefile").exists())
}

fn patch_device_name(profile_code: &str) -> &'static str {
    if profile_code == "z" {
        "zelda"
    } else {
        "mario"
    }
}

pub(crate) fn patch_game_watch_patch_workspace_for_modern_gcc(
    workspace_dir: &Path,
    device: &str,
) -> Result<(), String> {
    let main_c = workspace_dir.join("Core").join("Src").join("main.c");
    let original = fs::read_to_string(&main_c)
        .map_err(|error| format!("failed to read {}: {error}", main_c.display()))?;
    let patched = original
        .replace(
            "uint32_t *target_address;",
            "uint32_t target_address;",
        )
        .replace(
            "uint32_t sp = *target_address;",
            "uint32_t sp = *((uint32_t *)target_address);",
        )
        .replace(
            "uint32_t pc = *(target_address + 1);",
            "uint32_t pc = *((uint32_t *)target_address + 1);",
        );
    if patched != original {
        fs::write(&main_c, patched)
            .map_err(|error| format!("failed to patch {}: {error}", main_c.display()))?;
    }

    let makefile = workspace_dir.join("Makefile");
    let makefile_original = fs::read_to_string(&makefile)
        .map_err(|error| format!("failed to read {}: {error}", makefile.display()))?;
    let device_upper = device.to_ascii_uppercase();
    let device_lower = device.to_ascii_lowercase();
    let makefile_patched = makefile_original
        .replace(
            "GNW_DEVICE := $(shell $(PYTHON) -m scripts.device_from_patch_params $(PATCH_PARAMS))",
            &format!("GNW_DEVICE := {device_upper}"),
        )
        .replace(
            "GNW_DEVICE := $(shell \"$(PYTHON)\" -m scripts.device_from_patch_params $(PATCH_PARAMS))",
            &format!("GNW_DEVICE := {device_upper}"),
        )
        .replace(
            "GNW_DEVICE_LOWER := $(shell echo \"$(GNW_DEVICE)\" | tr 'A-Z' 'a-z')",
            &format!("GNW_DEVICE_LOWER := {device_lower}"),
        )
        .replace(
            "\t$(PYTHON) scripts/check_env_vars.py",
            "\t\"$(PYTHON)\" scripts/check_env_vars.py",
        )
        .replace(
            "\t\"$(PYTHON)\" scripts/check_env_vars.py",
            "\t\"$(PYTHON)\" -c \"import sys,runpy; sys.path.insert(0,'.'); runpy.run_module('scripts.device_from_patch_params', run_name='__main__')\" $(PATCH_PARAMS)\n\t\"$(PYTHON)\" scripts/check_env_vars.py",
        )
        .replace(
            "\t$(PYTHON) patch.py",
            "\t\"$(PYTHON)\" patch.py",
        )
        .replace(
            "\t\"$(PYTHON)\" patch.py",
            "\t\"$(PYTHON)\" -c \"import sys,runpy; sys.path.insert(0,'.'); runpy.run_path('patch.py', run_name='__main__')\"",
        )
        .replace(
            "\t@$(PYTHON) patch.py",
            "\t@\"$(PYTHON)\" patch.py",
        )
        .replace(
            "\t@\"$(PYTHON)\" patch.py",
            "\t@\"$(PYTHON)\" -c \"import sys,runpy; sys.path.insert(0,'.'); runpy.run_path('patch.py', run_name='__main__')\"",
        );
    if makefile_patched != makefile_original {
        fs::write(&makefile, makefile_patched)
            .map_err(|error| format!("failed to patch {}: {error}", makefile.display()))?;
    }
    Ok(())
}

pub(crate) fn run_game_watch_patch_build(
    app: &tauri::AppHandle,
    workspace_dir: &PathBuf,
    stock_bank1_path: &Path,
    stock_spi_path: &Path,
    profile_code: &str,
) -> Result<PathBuf, String> {
    let make_exe = locate_make_exe().ok_or_else(|| "mingw32-make.exe not found".to_string())?;
    let gcc_bin = locate_gcc_bin_dir().ok_or_else(|| "arm-none-eabi toolchain not found".to_string())?;
    let git_bin = locate_git_bin_dir().ok_or_else(|| "Git bash/sh not found".to_string())?;
    let patch_repo = locate_game_watch_patch_repo().ok_or_else(|| "game-and-watch-patch repository not found".to_string())?;
    validate_patch_stock_inputs(stock_bank1_path, stock_spi_path, profile_code)?;

    ensure_clean_dir(workspace_dir)?;
    copy_dir_filtered(&patch_repo, workspace_dir)?;

    let device = patch_device_name(profile_code);
    patch_game_watch_patch_workspace_for_modern_gcc(workspace_dir, device)?;
    copy_file_prefix(
        stock_bank1_path,
        &workspace_dir.join(format!("internal_flash_backup_{device}.bin")),
        128 * 1024,
    )?;
    let stock_spi_prefix_size = if profile_code == "z" {
        4 * 1024 * 1024
    } else {
        1024 * 1024
    };
    copy_file_prefix(
        stock_spi_path,
        &workspace_dir.join(format!("flash_backup_{device}.bin")),
        stock_spi_prefix_size,
    )?;

    let build_log_path = workspace_dir.join("build_gw_studio_patch.log");
    let bash_exe = git_bin.join("bash.exe");
    let make_dir = make_exe
        .parent()
        .ok_or_else(|| "failed to resolve mingw32-make.exe dir".to_string())?;
    let python_exe = locate_python_exe();
    let patch_params = if profile_code == "z" {
        "--device=zelda"
    } else {
        "--device=mario --internal-only"
    };
    let jobs = build_job_count();
    ensure_msys_tmp_dir(&git_bin)?;
    let bash_command = format!(
        "export PATH=\"{}:{}:{}:$PATH\"; cd \"{}\"; \"{}\" SHELL=\"{}\" -j{} PYTHON=\"{}\" PATCH_PARAMS=\"{}\" build/internal_flash_patched.bin 2>&1",
        to_bash_path_for(make_dir, &git_bin),
        to_bash_path_for(&gcc_bin, &git_bin),
        to_bash_path_for(&git_bin, &git_bin),
        to_bash_path_for(workspace_dir, &git_bin),
        to_bash_path_for(&make_exe, &git_bin),
        to_bash_path_for(&bash_exe, &git_bin),
        jobs,
        to_bash_path_for(&python_exe, &git_bin),
        patch_params,
    );

    emit_build_progress(app, 60.0, "Patching stock Bank1 for dualboot");
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
        .map_err(|error| format!("failed to launch game-and-watch-patch make: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture game-and-watch-patch output".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut combined = String::new();
    let mut progress = 60.0_f64;
    let mut last_emitted = -1_i32;

    loop {
        let line = read_lossy_output_line(&mut reader)
            .map_err(|error| format!("failed reading game-and-watch-patch output: {error}"))?;
        let Some(line) = line else {
            break;
        };
        combined.push_str(&line);
        combined.push('\n');

        if line.contains("BEGINING BINARY PATCH") || line.contains("BEGINNING BINARY PATCH") {
            progress = 62.0;
        } else if line.contains("Binary Patching Complete") {
            progress = 66.0;
        } else if line.contains("[ CC ") {
            progress = (progress + 0.15).min(65.0);
        }

        let rounded = progress.round() as i32;
        if rounded != last_emitted {
            last_emitted = rounded;
            emit_build_progress(app, progress, line.clone());
        }
    }

    let status = child
        .wait()
        .map_err(|error| format!("failed to wait for game-and-watch-patch build: {error}"))?;
    fs::write(&build_log_path, &combined)
        .map_err(|error| format!("failed to write Bank1 patch log: {error}"))?;

    if !status.success() {
        let tail = combined
            .lines()
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "Bank1 dualboot patch failed; see {}\n{}",
            build_log_path.to_string_lossy(),
            tail
        ));
    }

    let patched_bank1 = workspace_dir.join("build").join("internal_flash_patched.bin");
    if !patched_bank1.exists() {
        return Err(format!(
            "Bank1 dualboot patch did not create {}",
            patched_bank1.display()
        ));
    }
    Ok(patched_bank1)
}
