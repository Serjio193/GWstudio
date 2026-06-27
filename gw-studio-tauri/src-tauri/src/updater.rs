use crate::paths::host_root;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::fs;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
    signature_url: String,
    version: String,
}

#[derive(Deserialize)]
pub(crate) struct AppUpdateCheckRequest {
    current_version: String,
}

#[derive(Serialize)]
pub(crate) struct AppUpdateCheckResult {
    version: String,
    download_url: String,
    signature_url: String,
    sha256: String,
    release_url: String,
    is_newer: bool,
}

#[derive(Deserialize)]
struct GithubReleaseAsset {
    name: Option<String>,
    browser_download_url: Option<String>,
    digest: Option<String>,
}

#[derive(Deserialize)]
struct GithubLatestRelease {
    tag_name: Option<String>,
    name: Option<String>,
    html_url: Option<String>,
    assets: Option<Vec<GithubReleaseAsset>>,
}

const RELEASE_SIGNATURE_NAMESPACE: &str = "gwstudio-release";
const RELEASE_PUBLIC_KEY_B64: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIOhA2J9ebY5gZfLfMJ+0uFEBL/QFWab74GLqEG6nOq3u";
const RELEASE_EXE_ASSET_NAME: &str = "GWStudio.exe";
const RELEASE_SHA256_ASSET_NAME: &str = "GWStudio.exe.sha256";
const RELEASE_SIGNATURE_ASSET_NAME: &str = "GWStudio.exe.sig";
const GITHUB_LATEST_RELEASE_API: &str = "https://api.github.com/repos/Serjio193/GWstudio/releases/latest";

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

fn update_url_file_name(url: &str) -> &str {
    url.split('?')
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap_or_default()
}

fn allowed_update_exe_url(url: &str) -> bool {
    allowed_update_download_url(url) && update_url_file_name(url).eq_ignore_ascii_case(RELEASE_EXE_ASSET_NAME)
}

fn allowed_update_signature_url(url: &str) -> bool {
    allowed_update_download_url(url) && update_url_file_name(url).eq_ignore_ascii_case(RELEASE_SIGNATURE_ASSET_NAME)
}

fn parse_version(value: &str) -> Vec<u64> {
    value
        .trim()
        .trim_start_matches(|ch| ch == 'v' || ch == 'V')
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parts = parse_version(left);
    let right_parts = parse_version(right);
    let len = left_parts.len().max(right_parts.len()).max(3);
    for index in 0..len {
        let left_value = left_parts.get(index).copied().unwrap_or(0);
        let right_value = right_parts.get(index).copied().unwrap_or(0);
        match left_value.cmp(&right_value) {
            std::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    std::cmp::Ordering::Equal
}

fn parse_sha256_text(text: &str) -> Option<String> {
    text.split(|ch: char| !ch.is_ascii_hexdigit())
        .find(|part| part.len() == 64 && part.chars().all(|ch| ch.is_ascii_hexdigit()))
        .map(|part| part.to_ascii_lowercase())
}

fn github_latest_release_url() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("{GITHUB_LATEST_RELEASE_API}?t={timestamp}")
}

fn powershell_json_string(value: &str) -> String {
    let escaped = value.replace('`', "``").replace('"', "`\"");
    format!("\"{escaped}\"")
}

fn fetch_latest_release_json() -> Result<String, String> {
    let url = github_latest_release_url();
    let command_text = format!(
        "$ProgressPreference='SilentlyContinue'; \
         $headers=@{{Accept='application/vnd.github+json';'Cache-Control'='no-cache';'User-Agent'='GWStudio-Updater'}}; \
         Invoke-RestMethod -Headers $headers -Uri {} | ConvertTo-Json -Depth 20 -Compress",
        powershell_json_string(&url)
    );
    let mut command = Command::new("powershell");
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(command_text)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    let output = hide_command_window(&mut command)
        .output()
        .map_err(|error| format!("failed to start update metadata request: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("update metadata request failed with status {}", output.status)
        } else {
            format!("update metadata request failed: {stderr}")
        });
    }
    String::from_utf8(output.stdout).map_err(|error| format!("update metadata is not UTF-8: {error}"))
}

fn find_release_asset<'a>(
    assets: &'a [GithubReleaseAsset],
    name: &str,
) -> Option<&'a GithubReleaseAsset> {
    assets
        .iter()
        .find(|asset| asset.name.as_deref().unwrap_or_default().eq_ignore_ascii_case(name))
}

fn read_ssh_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, String> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| "SSH signature offset overflow".to_string())?;
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| "truncated SSH signature field length".to_string())?;
    *offset = end;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_ssh_string<'a>(bytes: &'a [u8], offset: &mut usize) -> Result<&'a [u8], String> {
    let len = read_ssh_u32(bytes, offset)? as usize;
    let end = offset
        .checked_add(len)
        .ok_or_else(|| "SSH signature field offset overflow".to_string())?;
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| "truncated SSH signature field".to_string())?;
    *offset = end;
    Ok(slice)
}

fn push_ssh_string(output: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    let len = u32::try_from(value.len())
        .map_err(|_| "SSH signature field is too large".to_string())?;
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

fn decode_release_public_key() -> Result<[u8; 32], String> {
    let public_blob = base64::engine::general_purpose::STANDARD
        .decode(RELEASE_PUBLIC_KEY_B64)
        .map_err(|error| format!("failed to decode release public key: {error}"))?;
    let mut offset = 0;
    let key_type = read_ssh_string(&public_blob, &mut offset)?;
    if key_type != b"ssh-ed25519" {
        return Err("release public key is not ssh-ed25519".to_string());
    }
    let key = read_ssh_string(&public_blob, &mut offset)?;
    if offset != public_blob.len() {
        return Err("release public key has trailing data".to_string());
    }
    key.try_into()
        .map_err(|_| "release public key has invalid Ed25519 length".to_string())
}

fn decode_armored_ssh_signature(text: &str) -> Result<Vec<u8>, String> {
    let mut inside = false;
    let mut encoded = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "-----BEGIN SSH SIGNATURE-----" {
            inside = true;
            continue;
        }
        if trimmed == "-----END SSH SIGNATURE-----" {
            break;
        }
        if inside {
            encoded.push_str(trimmed);
        }
    }
    if encoded.is_empty() {
        return Err("update signature is missing SSH signature armor".to_string());
    }
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("failed to decode update signature: {error}"))
}

fn verify_update_signature(update_exe: &Path, signature_path: &Path) -> Result<(), String> {
    let signature_text = fs::read_to_string(signature_path)
        .map_err(|error| format!("failed to read update signature {}: {error}", signature_path.display()))?;
    let signature_blob = decode_armored_ssh_signature(&signature_text)?;
    let mut offset = 0;
    if signature_blob.get(0..6) != Some(b"SSHSIG") {
        return Err("update signature is not an OpenSSH SSHSIG blob".to_string());
    }
    offset += 6;
    let version = read_ssh_u32(&signature_blob, &mut offset)?;
    if version != 1 {
        return Err(format!("unsupported SSH signature version: {version}"));
    }
    let public_key_blob = read_ssh_string(&signature_blob, &mut offset)?;
    let namespace = read_ssh_string(&signature_blob, &mut offset)?;
    let reserved = read_ssh_string(&signature_blob, &mut offset)?;
    let hash_algorithm = read_ssh_string(&signature_blob, &mut offset)?;
    let signature_wrapper = read_ssh_string(&signature_blob, &mut offset)?;
    if offset != signature_blob.len() {
        return Err("update signature has trailing data".to_string());
    }
    if namespace != RELEASE_SIGNATURE_NAMESPACE.as_bytes() {
        return Err("update signature namespace is invalid".to_string());
    }
    if !reserved.is_empty() {
        return Err("update signature reserved field is not empty".to_string());
    }
    if hash_algorithm != b"sha512" {
        return Err("update signature must use sha512".to_string());
    }

    let release_public_key = decode_release_public_key()?;
    if public_key_blob != base64::engine::general_purpose::STANDARD
        .decode(RELEASE_PUBLIC_KEY_B64)
        .map_err(|error| format!("failed to decode release public key: {error}"))?
        .as_slice()
    {
        return Err("update signature was made with an unknown release key".to_string());
    }

    let mut signature_offset = 0;
    let signature_type = read_ssh_string(signature_wrapper, &mut signature_offset)?;
    let signature_bytes = read_ssh_string(signature_wrapper, &mut signature_offset)?;
    if signature_offset != signature_wrapper.len() {
        return Err("update signature wrapper has trailing data".to_string());
    }
    if signature_type != b"ssh-ed25519" {
        return Err("update signature algorithm is not ssh-ed25519".to_string());
    }
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "update signature has invalid Ed25519 length".to_string())?;
    let signature = Signature::from_bytes(&signature_array);

    let update_bytes = fs::read(update_exe)
        .map_err(|error| format!("failed to read downloaded update for signature check: {error}"))?;
    let update_hash = Sha512::digest(&update_bytes);
    let mut signed_data = Vec::new();
    push_ssh_string(&mut signed_data, b"SSHSIG")?;
    push_ssh_string(&mut signed_data, RELEASE_SIGNATURE_NAMESPACE.as_bytes())?;
    push_ssh_string(&mut signed_data, b"")?;
    push_ssh_string(&mut signed_data, b"sha512")?;
    push_ssh_string(&mut signed_data, &update_hash)?;

    let verifying_key = VerifyingKey::from_bytes(&release_public_key)
        .map_err(|error| format!("failed to load release public key: {error}"))?;
    verifying_key
        .verify(&signed_data, &signature)
        .map_err(|error| format!("update signature verification failed: {error}"))
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

    let rollback_exe = target_exe.with_extension("exe.rollback");
    fs::copy(&target_exe, &rollback_exe).map_err(|error| {
        format!(
            "failed to create update rollback copy {}: {error}",
            rollback_exe.display()
        )
    })?;

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
        .map_err(|error| {
            let _ = move_file_replace(&rollback_exe, &target_exe);
            format!("failed to restart updated app; rollback restored if possible: {error}")
        })?;

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
pub(crate) async fn check_app_update(
    request: AppUpdateCheckRequest,
) -> Result<AppUpdateCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let release_json = fetch_latest_release_json()?;
        let release: GithubLatestRelease = serde_json::from_str(&release_json)
            .map_err(|error| format!("failed to parse update metadata: {error}"))?;
        let latest_version = release
            .tag_name
            .or(release.name)
            .unwrap_or_default()
            .trim()
            .trim_start_matches(|ch| ch == 'v' || ch == 'V')
            .to_string();
        if latest_version.is_empty() {
            return Err("latest release version is empty".to_string());
        }

        let assets = release.assets.unwrap_or_default();
        let exe_asset = find_release_asset(&assets, RELEASE_EXE_ASSET_NAME)
            .ok_or_else(|| format!("latest release does not contain {RELEASE_EXE_ASSET_NAME}"))?;
        let sha_asset = find_release_asset(&assets, RELEASE_SHA256_ASSET_NAME)
            .ok_or_else(|| format!("latest release does not contain {RELEASE_SHA256_ASSET_NAME}"))?;
        let sig_asset = find_release_asset(&assets, RELEASE_SIGNATURE_ASSET_NAME)
            .ok_or_else(|| format!("latest release does not contain {RELEASE_SIGNATURE_ASSET_NAME}"))?;

        let download_url = exe_asset
            .browser_download_url
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string();
        let signature_url = sig_asset
            .browser_download_url
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string();
        if !allowed_update_exe_url(&download_url) {
            return Err("latest release executable URL is not allowed".to_string());
        }
        if !allowed_update_signature_url(&signature_url) {
            return Err("latest release signature URL is not allowed".to_string());
        }

        let mut sha256 = exe_asset
            .digest
            .as_deref()
            .and_then(parse_sha256_text)
            .or_else(|| sha_asset.digest.as_deref().and_then(parse_sha256_text))
            .unwrap_or_default();
        if sha256.is_empty() {
            let sha_url = sha_asset
                .browser_download_url
                .as_deref()
                .unwrap_or_default()
                .trim();
            if !allowed_update_download_url(sha_url)
                || !update_url_file_name(sha_url).eq_ignore_ascii_case(RELEASE_SHA256_ASSET_NAME)
            {
                return Err("latest release SHA256 URL is not allowed".to_string());
            }
            let update_dir = host_root().join("GWStudioUpdate");
            fs::create_dir_all(&update_dir).map_err(|error| {
                format!("failed to create update dir {}: {error}", update_dir.display())
            })?;
            let sha_path = update_dir.join("latest-update.sha256");
            download_file(sha_url, &sha_path)?;
            let sha_text = fs::read_to_string(&sha_path)
                .map_err(|error| format!("failed to read downloaded SHA256 asset: {error}"))?;
            sha256 = parse_sha256_text(&sha_text)
                .ok_or_else(|| "latest release SHA256 asset is invalid".to_string())?;
        }
        if sha256.len() != 64 || !sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err("latest release SHA256 asset is invalid".to_string());
        }

        Ok(AppUpdateCheckResult {
            version: latest_version.clone(),
            download_url,
            signature_url,
            sha256,
            release_url: release.html_url.unwrap_or_else(|| {
                "https://github.com/Serjio193/GWstudio".to_string()
            }),
            is_newer: compare_versions(&latest_version, &request.current_version)
                == std::cmp::Ordering::Greater,
        })
    })
    .await
    .map_err(|error| format!("failed to join update check task: {error}"))?
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
        if !allowed_update_exe_url(download_url) {
            return Err("update download URL is not allowed".to_string());
        }
        let signature_url = request.signature_url.trim();
        if signature_url.is_empty() {
            return Err("update signature URL is required".to_string());
        }
        if !allowed_update_signature_url(signature_url) {
            return Err("update signature URL is not allowed".to_string());
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
        let update_sig = update_dir.join(format!("GW Studio-{version_safe}.update.exe.sig"));
        download_file(download_url, &update_exe)?;
        download_file(signature_url, &update_sig)?;
        let metadata = fs::metadata(&update_exe)
            .map_err(|error| format!("failed to stat downloaded update: {error}"))?;
        if metadata.len() < 10 * 1024 * 1024 {
            return Err(format!("downloaded update is unexpectedly small: {} bytes", metadata.len()));
        }

        let normalized_expected = request
            .expected_sha256
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if normalized_expected.len() != 64 || !normalized_expected.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err("valid update SHA256 is required".to_string());
        }
        let bytes = fs::read(&update_exe)
            .map_err(|error| format!("failed to read downloaded update: {error}"))?;
        let actual_hash = format!("{:x}", Sha256::digest(&bytes));
        if actual_hash != normalized_expected {
            return Err(format!(
                "update SHA256 mismatch: expected {normalized_expected}, got {actual_hash}"
            ));
        }
        verify_update_signature(&update_exe, &update_sig)?;

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
