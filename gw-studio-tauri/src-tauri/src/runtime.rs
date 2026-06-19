use crate::paths::{
    current_portable_runtime_root, host_root, set_portable_runtime_root,
};
use serde::Serialize;
use std::fs;
use std::io::Cursor;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::CloseHandle;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
};

include!(concat!(env!("OUT_DIR"), "/portable_assets.rs"));

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize, Clone)]
struct PortableRuntimeProgressEvent {
    progress: f64,
    asset: String,
    message: String,
}

fn hide_command_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(target_os = "windows")]
fn wait_for_process_exit(pid: u32) {
    if pid == 0 {
        return;
    }
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(handle, INFINITE);
            let _ = CloseHandle(handle);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn wait_for_process_exit(_pid: u32) {}

fn create_portable_temp_dir() -> Result<PathBuf, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let dir = host_root()
        .join("GWStudioRuntime")
        .join(format!("{}-{}", std::process::id(), millis));
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create portable runtime dir {}: {error}", dir.display()))?;
    Ok(dir)
}

fn cleanup_stale_portable_runtime_dirs() {
    let root = host_root().join("GWStudioRuntime");
    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };
    let current_pid_prefix = format!("{}-", std::process::id());
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with(&current_pid_prefix) {
            continue;
        }
        let _ = fs::remove_dir_all(path);
    }
}

pub(crate) fn cleanup_current_portable_runtime_dir() {
    let Some(runtime_dir) = current_portable_runtime_root() else {
        return;
    };
    for _ in 0..12 {
        if !runtime_dir.exists() {
            break;
        }
        if fs::remove_dir_all(&runtime_dir).is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    let runtime_root = host_root().join("GWStudioRuntime");
    if runtime_root
        .read_dir()
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
    {
        let _ = fs::remove_dir(&runtime_root);
    }
}

pub(crate) fn handle_runtime_cleanup_helper_args() -> bool {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.get(1).and_then(|arg| arg.to_str()) != Some("--gw-studio-clean-runtime") {
        return false;
    }

    let parent_pid = args
        .get(2)
        .and_then(|value| value.to_str())
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let Some(runtime_dir) = args.get(3).map(PathBuf::from) else {
        std::process::exit(1);
    };
    let Some(runtime_root) = args.get(4).map(PathBuf::from) else {
        std::process::exit(1);
    };

    wait_for_process_exit(parent_pid);
    thread::sleep(Duration::from_millis(300));
    for _ in 0..20 {
        if !runtime_dir.exists() || fs::remove_dir_all(&runtime_dir).is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    if runtime_root
        .read_dir()
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
    {
        let _ = fs::remove_dir(&runtime_root);
    }
    std::process::exit(0);
}

pub(crate) fn spawn_runtime_cleanup_helper() {
    let Some(runtime_dir) = current_portable_runtime_root() else {
        return;
    };
    let runtime_root = host_root().join("GWStudioRuntime");
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    let mut command = Command::new(exe_path);
    command
        .arg("--gw-studio-clean-runtime")
        .arg(std::process::id().to_string())
        .arg(runtime_dir)
        .arg(runtime_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = hide_command_window(&mut command).spawn();
}

fn emit_portable_runtime_progress(
    app: &tauri::AppHandle,
    progress: f64,
    asset: &str,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "portable-runtime-progress",
        PortableRuntimeProgressEvent {
            progress: progress.clamp(0.0, 100.0),
            asset: asset.to_string(),
            message: message.into(),
        },
    );
}

fn extract_zip_bytes_to_dir(
    app: &tauri::AppHandle,
    asset_name: &str,
    bytes: &[u8],
    destination: &Path,
    progress_start: f64,
    progress_end: f64,
) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("failed to create portable asset dir {}: {error}", destination.display()))?;
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|error| format!("failed to open embedded portable zip: {error}"))?;
    let archive_len = archive.len();
    let total_entries = archive_len.max(1) as f64;

    for index in 0..archive_len {
        let mut file = archive
            .by_index(index)
            .map_err(|error| format!("failed to read embedded portable zip entry {index}: {error}"))?;
        let Some(enclosed_name) = file.enclosed_name() else {
            continue;
        };
        let output_path = destination.join(enclosed_name);
        if file.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("failed to create portable dir {}: {error}", output_path.display()))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create portable dir {}: {error}", parent.display()))?;
        }
        let mut output = fs::File::create(&output_path)
            .map_err(|error| format!("failed to create portable file {}: {error}", output_path.display()))?;
        std::io::copy(&mut file, &mut output)
            .map_err(|error| format!("failed to extract portable file {}: {error}", output_path.display()))?;
        if index == 0 || index % 80 == 0 || index + 1 == archive_len {
            let local_progress = ((index + 1) as f64 / total_entries).clamp(0.0, 1.0);
            let progress = progress_start + ((progress_end - progress_start) * local_progress);
            emit_portable_runtime_progress(
                app,
                progress,
                asset_name,
                format!("Extracting {asset_name}: {}/{}", index + 1, archive_len),
            );
        }
    }
    Ok(())
}

pub(crate) fn prepare_portable_runtime(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    if PORTABLE_ASSETS.is_empty() {
        emit_portable_runtime_progress(app, 100.0, "runtime", "portable runtime not bundled");
        return Ok(None);
    }

    cleanup_stale_portable_runtime_dirs();
    let runtime_dir = create_portable_temp_dir()?;
    set_portable_runtime_root(runtime_dir.clone());
    let total_assets = PORTABLE_ASSETS.len().max(1) as f64;
    for (asset_index, (asset_name, target_dir, bytes)) in PORTABLE_ASSETS.iter().enumerate() {
        let progress_start = (asset_index as f64 / total_assets) * 100.0;
        let progress_end = ((asset_index + 1) as f64 / total_assets) * 100.0;
        let destination = runtime_dir.join(target_dir);
        emit_portable_runtime_progress(
            app,
            progress_start,
            asset_name,
            format!("Starting {asset_name}"),
        );
        extract_zip_bytes_to_dir(app, asset_name, bytes, &destination, progress_start, progress_end)
            .map_err(|error| format!("failed to extract embedded {asset_name}: {error}"))?;
    }
    emit_portable_runtime_progress(app, 100.0, "runtime", "portable runtime ready");
    Ok(Some(runtime_dir))
}
