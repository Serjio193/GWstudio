use crate::paths::host_root;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::CloseHandle;
#[cfg(target_os = "windows")]
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::Urlmon::URLDownloadToFileW;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Deserialize)]
pub(crate) struct OpenExternalUrlRequest {
    url: String,
}

#[derive(Deserialize)]
pub(crate) struct AppUpdateInstallRequest {
    download_url: String,
    expected_sha256: Option<String>,
    version: String,
}

fn hide_command_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(target_os = "windows")]
fn to_wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(std::iter::once(0)).collect()
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

#[cfg(target_os = "windows")]
fn download_file(url: &str, destination: &Path) -> Result<(), String> {
    let url_wide = to_wide(url);
    let destination_wide = to_wide(destination.as_os_str());
    unsafe {
        URLDownloadToFileW(
            None,
            PCWSTR(url_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            0,
            None,
        )
    }
    .map_err(|error| format!("update download failed: {error}"))
}

#[cfg(not(target_os = "windows"))]
fn download_file(_url: &str, _destination: &Path) -> Result<(), String> {
    Err("app update download is only implemented on Windows".to_string())
}

#[cfg(target_os = "windows")]
fn move_file_replace(source: &Path, destination: &Path) -> Result<(), String> {
    let source_wide = to_wide(source.as_os_str());
    let destination_wide = to_wide(destination.as_os_str());
    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING,
        )
    }
    .map_err(|error| {
        format!(
            "failed to replace {} with {}: {error}",
            destination.display(),
            source.display()
        )
    })
}

#[cfg(not(target_os = "windows"))]
fn move_file_replace(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(|error| {
        format!(
            "failed to replace {} with {}: {error}",
            destination.display(),
            source.display()
        )
    })
}

fn allowed_external_url(url: &str) -> bool {
    matches!(
        url,
        "https://github.com/Serjio193/GWstudio"
            | "https://github.com/Serjio193/GWstudio/"
            | "https://www.paypal.com/paypalme/SerhiiTarnopovych"
    )
}

fn allowed_update_download_url(url: &str) -> bool {
    url.starts_with("https://github.com/Serjio193/GWstudio/releases/download/")
}

pub(crate) fn cleanup_stale_update_dir() {
    let update_dir = host_root().join("GWStudioUpdate");
    if update_dir.exists() {
        let _ = fs::remove_dir_all(update_dir);
    }
}

pub(crate) fn handle_update_helper_args() -> bool {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.get(1).and_then(|arg| arg.to_str()) != Some("--gw-studio-apply-update") {
        return false;
    }

    let exit_code = match run_update_helper(&args) {
        Ok(()) => 0,
        Err(error) => {
            let error_dir = args
                .get(6)
                .map(PathBuf::from)
                .unwrap_or_else(|| host_root().join("GWStudioUpdate"));
            let _ = fs::create_dir_all(&error_dir);
            let _ = fs::write(
                error_dir.join("apply_update_error.log"),
                error,
            );
            1
        }
    };
    std::process::exit(exit_code);
}

fn run_update_helper(args: &[std::ffi::OsString]) -> Result<(), String> {
    let parent_pid = args
        .get(2)
        .and_then(|value| value.to_str())
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| "missing update helper parent pid".to_string())?;
    let update_exe = PathBuf::from(
        args.get(3)
            .ok_or_else(|| "missing update helper source exe".to_string())?,
    );
    let target_exe = PathBuf::from(
        args.get(4)
            .ok_or_else(|| "missing update helper target exe".to_string())?,
    );
    let working_dir = PathBuf::from(
        args.get(5)
            .ok_or_else(|| "missing update helper working dir".to_string())?,
    );
    let update_dir = PathBuf::from(
        args.get(6)
            .ok_or_else(|| "missing update helper update dir".to_string())?,
    );

    wait_for_process_exit(parent_pid);
    thread::sleep(Duration::from_millis(500));

    let mut last_error = None;
    for _ in 0..40 {
        match move_file_replace(&update_exe, &target_exe) {
            Ok(()) => {
                last_error = None;
                break;
            }
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(250));
            }
        }
    }
    if let Some(error) = last_error {
        return Err(error);
    }

    let mut command = Command::new(&target_exe);
    command
        .current_dir(&working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_command_window(&mut command)
        .spawn()
        .map_err(|error| format!("failed to restart updated app: {error}"))?;

    let _ = fs::remove_file(update_exe);
    let _ = fs::remove_dir_all(update_dir);
    Ok(())
}

#[tauri::command]
pub(crate) fn app_sha256() -> Result<String, String> {
    let exe_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current exe path: {error}"))?;
    let bytes = fs::read(&exe_path)
        .map_err(|error| format!("failed to read current exe {}: {error}", exe_path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

#[tauri::command]
pub(crate) fn open_external_url(request: OpenExternalUrlRequest) -> Result<(), String> {
    let url = request.url.trim();
    if !allowed_external_url(url) {
        return Err("external URL is not allowed".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("rundll32.exe");
        command
            .arg("url.dll,FileProtocolHandler")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_command_window(&mut command)
            .spawn()
            .map_err(|error| format!("failed to open external URL: {error}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut command = Command::new("xdg-open");
        command
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
            .spawn()
            .map_err(|error| format!("failed to open external URL: {error}"))?;
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn install_app_update(
    app: tauri::AppHandle,
    request: AppUpdateInstallRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let download_url = request.download_url.trim();
        if !allowed_update_download_url(download_url) {
            return Err("update download URL is not allowed".to_string());
        }

        let exe_path = std::env::current_exe()
            .map_err(|error| format!("failed to resolve current exe path: {error}"))?;
        let exe_dir = exe_path
            .parent()
            .ok_or_else(|| "failed to resolve current exe dir".to_string())?
            .to_path_buf();
        let update_dir = host_root().join("GWStudioUpdate");
        cleanup_stale_update_dir();
        fs::create_dir_all(&update_dir)
            .map_err(|error| format!("failed to create update dir {}: {error}", update_dir.display()))?;

        let version_safe = request
            .version
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
            .collect::<String>();
        let update_exe = update_dir.join(format!("GW Studio-{version_safe}.update.exe"));
        download_file(download_url, &update_exe)?;
        let metadata = fs::metadata(&update_exe)
            .map_err(|error| format!("failed to stat downloaded update: {error}"))?;
        if metadata.len() < 10 * 1024 * 1024 {
            return Err(format!("downloaded update is unexpectedly small: {} bytes", metadata.len()));
        }

        if let Some(expected_hash) = request.expected_sha256.as_deref() {
            let normalized_expected = expected_hash.trim().to_ascii_lowercase();
            if normalized_expected.len() == 64 && normalized_expected.chars().all(|ch| ch.is_ascii_hexdigit()) {
                let bytes = fs::read(&update_exe)
                    .map_err(|error| format!("failed to read downloaded update: {error}"))?;
                let actual_hash = format!("{:x}", Sha256::digest(&bytes));
                if actual_hash != normalized_expected {
                    return Err(format!(
                        "update SHA256 mismatch: expected {normalized_expected}, got {actual_hash}"
                    ));
                }
            }
        }

        let helper_exe = update_dir.join("GWStudioUpdateHelper.exe");
        fs::copy(&exe_path, &helper_exe)
            .map_err(|error| format!("failed to prepare update helper: {error}"))?;

        let mut apply_command = Command::new(&helper_exe);
        apply_command
            .arg("--gw-studio-apply-update")
            .arg(std::process::id().to_string())
            .arg(&update_exe)
            .arg(&exe_path)
            .arg(&exe_dir)
            .arg(&update_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_command_window(&mut apply_command)
            .spawn()
            .map_err(|error| format!("failed to start update installer: {error}"))?;

        app.exit(0);
        Ok(())
    })
    .await
    .map_err(|error| format!("failed to join update task: {error}"))?
}
