use crate::paths::host_root;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

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

fn ps_single_quote_text(value: &str) -> String {
    value.replace('\'', "''")
}

fn ps_single_quote(value: &Path) -> String {
    value.to_string_lossy().replace('\'', "''")
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
        fs::create_dir_all(&update_dir)
            .map_err(|error| format!("failed to create update dir {}: {error}", update_dir.display()))?;

        let version_safe = request
            .version
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
            .collect::<String>();
        let update_exe = update_dir.join(format!("GW Studio-{version_safe}.update.exe"));

        let download_script = format!(
            "$ErrorActionPreference='Stop'; \
             $ProgressPreference='SilentlyContinue'; \
             [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
            ps_single_quote_text(download_url),
            ps_single_quote(&update_exe),
        );
        let mut download_command = Command::new("powershell.exe");
        download_command
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(download_script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = hide_command_window(&mut download_command)
            .output()
            .map_err(|error| format!("failed to start update download: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "update download failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
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

        let script_path = update_dir.join("apply_update.ps1");
        let script = format!(
            "$ErrorActionPreference='Stop'\n\
             Wait-Process -Id {} -ErrorAction SilentlyContinue\n\
             Start-Sleep -Milliseconds 500\n\
             Move-Item -LiteralPath '{}' -Destination '{}' -Force\n\
             Start-Process -FilePath '{}' -WorkingDirectory '{}'\n\
             Start-Sleep -Milliseconds 300\n\
             Remove-Item -LiteralPath '{}' -Force -ErrorAction SilentlyContinue\n",
            std::process::id(),
            ps_single_quote(&update_exe),
            ps_single_quote(&exe_path),
            ps_single_quote(&exe_path),
            ps_single_quote(&exe_dir),
            ps_single_quote(&script_path),
        );
        fs::write(&script_path, script)
            .map_err(|error| format!("failed to write update script: {error}"))?;

        let mut apply_command = Command::new("powershell.exe");
        apply_command
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-File")
            .arg(&script_path)
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
