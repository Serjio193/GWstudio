use base64::Engine;
use image::imageops::FilterType;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tauri::WindowEvent;

include!(concat!(env!("OUT_DIR"), "/portable_assets.rs"));

static PORTABLE_RUNTIME_ROOT: OnceLock<PathBuf> = OnceLock::new();

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize)]
struct RuntimeStatus {
    workspace_root: String,
    logs_dir: String,
    tools_dir: String,
    thumbnails_dir: String,
    host_root: String,
    gnwmanager_source: String,
    rust_backend: &'static str,
}

#[derive(Serialize, Clone)]
struct PortableRuntimeReadyEvent {
    ok: bool,
    runtime_dir: String,
    message: String,
}

#[derive(Serialize, Clone)]
struct PortableRuntimeProgressEvent {
    progress: f64,
    asset: String,
    message: String,
}

#[derive(Deserialize)]
struct ThumbnailCacheRequest {
    emulator: String,
    title: String,
}

#[derive(Deserialize)]
struct ThumbnailSaveRequest {
    emulator: String,
    title: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct BinaryFilePathRequest {
    path: String,
}

#[derive(Deserialize)]
struct RevealPathRequest {
    path: String,
}

#[derive(Deserialize)]
struct OpenExternalUrlRequest {
    url: String,
}

#[derive(Deserialize)]
struct AppUpdateInstallRequest {
    download_url: String,
    expected_sha256: Option<String>,
    version: String,
}

#[derive(Deserialize)]
struct BinFilePickerRequest {
    title: String,
    default_path: Option<String>,
}

#[derive(Serialize)]
struct BinFilePickerResult {
    name: String,
    path: String,
}

#[derive(Deserialize)]
struct StockBackupImportRequest {
    firmware_profile: String,
    kind: String,
    path: String,
}

#[derive(Serialize)]
struct StockBackupImportResult {
    name: String,
    path: String,
    size_bytes: u64,
}

#[derive(Deserialize)]
struct DeviceBackupLookupRequest {
    device_uid: String,
    firmware_profile: Option<String>,
}

#[derive(Serialize)]
struct DeviceBackupLookupResult {
    mcu_name: String,
    mcu_path: String,
    bank2_name: String,
    bank2_path: String,
    spi_name: String,
    spi_path: String,
}

#[derive(Deserialize)]
struct RomImportFile {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct MsxBiosPathsRequest {
    paths: Vec<String>,
}

#[derive(Deserialize)]
struct MsxBiosFilesRequest {
    files: Vec<RomImportFile>,
}

#[derive(Serialize)]
struct MsxBiosStatus {
    ready: bool,
    dir: String,
    missing: Vec<String>,
}

#[derive(Deserialize)]
struct RomImportRequest {
    emulator: String,
    files: Vec<RomImportFile>,
}

#[derive(Serialize)]
struct RomImportEntry {
    title: String,
    path: String,
    size_bytes: u64,
}

#[derive(Serialize)]
struct RomImportResult {
    entries: Vec<RomImportEntry>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct AutoRomImportEntry {
    emulator: String,
    title: String,
    path: String,
    size_bytes: u64,
}

#[derive(Serialize)]
struct AutoRomImportResult {
    entries: Vec<AutoRomImportEntry>,
    warnings: Vec<String>,
}

#[derive(Deserialize)]
struct RomImportPathsRequest {
    paths: Vec<String>,
}

#[derive(Deserialize)]
struct BuildMetricsRequest {
    emulator: String,
    titles: Vec<String>,
    rom_paths: Vec<String>,
}

#[derive(Deserialize)]
struct BuildBundleEntry {
    emulator: String,
    title: String,
    rom_path: String,
}

#[derive(Deserialize)]
struct BuildBundleRequest {
    firmware_profile: String,
    installed_spi_mb: f64,
    firmware_reserved_mb: f64,
    stock_bank1_path: Option<String>,
    stock_spi_path: Option<String>,
    coverflow_enabled: Option<bool>,
    entries: Vec<BuildBundleEntry>,
}

#[derive(Serialize, Clone)]
struct BuildProgressEvent {
    progress: f64,
    message: String,
}

#[derive(Serialize)]
struct BuildMetrics {
    rom_bytes: u64,
    image_bytes: u64,
    rom_count: usize,
    image_count: usize,
}

#[derive(Serialize)]
struct BuildBundleResult {
    bundle_dir: String,
    summary_path: String,
    manifest_path: String,
    retro_go_workspace_dir: String,
    build_log_path: String,
    extflash_built: bool,
    bank1_candidate_path: String,
    bank2_candidate_path: String,
    extflash_build_path: String,
    extflash_build_size_bytes: u64,
    rom_count: usize,
    image_count: usize,
    coverflow_count: usize,
    romart_count: usize,
    rom_bytes: u64,
    image_bytes: u64,
    total_bytes: u64,
}

#[derive(Serialize)]
struct FirmwareBundleLookupResult {
    found: bool,
    message: String,
    bundle_dir: String,
    manifest_path: String,
    bank1_candidate_path: String,
    bank2_candidate_path: String,
    extflash_build_path: String,
    extflash_build_size_bytes: u64,
}

#[derive(Serialize)]
struct DeviceInfo {
    summary: String,
    programmer: String,
    probe_vendor: String,
    probe_id: String,
    device_uid: String,
    cpu_id: String,
    target_voltage: String,
    mcu_profile: String,
    detected_firmware: String,
    external_flash: String,
    protection: String,
    filesystem: String,
}

static CURRENT_DEVICE_UID: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static SERVICE_BRIDGE_STATE: OnceLock<Mutex<ServiceBridgeState>> = OnceLock::new();

#[derive(Clone, Serialize, Deserialize)]
struct ServiceBridgeState {
    id: String,
    command: String,
    ok: bool,
    message: String,
    device_uid: String,
    programmer: String,
    probe_vendor: String,
    probe_id: String,
    cpu_id: String,
    target_voltage: String,
    mcu_profile: String,
    detected_firmware: String,
    external_flash: String,
    protection: String,
    filesystem: String,
    backup_ready: bool,
    updated_at: String,
}

#[derive(Deserialize)]
struct ServiceBridgeRequest {
    id: String,
    command: String,
}

#[derive(Serialize)]
struct ServiceBridgeResponse {
    id: String,
    command: String,
    ok: bool,
    message: String,
    state: ServiceBridgeState,
}

#[derive(Serialize)]
struct BackupReadResult {
    summary: String,
    path: String,
    name: String,
    backend: String,
    phase: String,
    frequency: u32,
    speed_bps: f64,
    stderr: String,
}

#[derive(Serialize)]
struct FirmwareWriteResult {
    summary: String,
    path: String,
    target: String,
    backend: String,
    frequency: u32,
    stderr: String,
}

#[derive(Clone, Serialize)]
struct FirmwareWriteProgressEvent {
    phase: String,
    stage: String,
    progress: f64,
    speed_bps: f64,
    frequency: u32,
    backend: String,
    message: String,
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

fn host_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn workspace_root() -> PathBuf {
    host_root().join("GameWatchBuilderData")
}

fn hide_command_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn portable_runtime_root() -> PathBuf {
    PORTABLE_RUNTIME_ROOT
        .get()
        .cloned()
        .unwrap_or_else(workspace_root)
}

fn runtime_tools_dir() -> PathBuf {
    portable_runtime_root().join("tools")
}

fn portable_source_dir() -> PathBuf {
    portable_runtime_root().join("sources")
}

fn tool_candidate(parts: &[&str]) -> PathBuf {
    parts
        .iter()
        .fold(runtime_tools_dir(), |path, part| path.join(part))
}

fn find_file_under(root: &Path, file_name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case(file_name))
                .unwrap_or(false)
            {
                return Some(path);
            }
        }
    }
    None
}

fn find_dir_with_files_under(root: &Path, required_files: &[&str]) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if required_files.iter().all(|file| dir.join(file).is_file()) {
            return Some(dir);
        }
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    None
}

fn contains_cyrillic(value: &str) -> bool {
    value.chars().any(|ch| matches!(ch, '\u{0400}'..='\u{04ff}' | '\u{0500}'..='\u{052f}' | '\u{2de0}'..='\u{2dff}' | '\u{a640}'..='\u{a69f}'))
}

fn validate_exe_path_for_portable_runtime() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve executable path: {error}"))?;
    let exe_path_text = exe_path.to_string_lossy();
    if contains_cyrillic(&exe_path_text) {
        return Err(format!(
            "GW Studio cannot run from a path containing Cyrillic characters.\n\nCurrent path:\n{}\n\nMove the program to a folder with Latin-only path, for example C:\\GWStudio\\, and start it again.\n\nGW Studio не может работать из папки с кириллицей. Переместите программу в папку без кириллицы, например C:\\GWStudio\\.",
            exe_path.display()
        ));
    }
    Ok(())
}

fn show_startup_error(message: &str) {
    #[cfg(target_os = "windows")]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

        let title = "GW Studio startup error"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let text = message
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        unsafe {
            let _ = MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("{message}");
    }
}

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

fn cleanup_current_portable_runtime_dir() {
    let Some(runtime_dir) = PORTABLE_RUNTIME_ROOT.get().cloned() else {
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

fn ps_single_quote(value: &Path) -> String {
    value
        .to_string_lossy()
        .replace('\'', "''")
}

fn spawn_runtime_cleanup_helper() {
    let Some(runtime_dir) = PORTABLE_RUNTIME_ROOT.get().cloned() else {
        return;
    };
    let runtime_root = host_root().join("GWStudioRuntime");
    let pid = std::process::id();
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue'; \
         Wait-Process -Id {pid}; \
         Start-Sleep -Milliseconds 300; \
         Remove-Item -LiteralPath '{}' -Recurse -Force; \
         if ((Test-Path -LiteralPath '{}') -and -not (Get-ChildItem -LiteralPath '{}' -Force)) {{ Remove-Item -LiteralPath '{}' -Force }}",
        ps_single_quote(&runtime_dir),
        ps_single_quote(&runtime_root),
        ps_single_quote(&runtime_root),
        ps_single_quote(&runtime_root),
    );
    let mut command = Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg(script)
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

fn prepare_portable_runtime(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    if PORTABLE_ASSETS.is_empty() {
        emit_portable_runtime_progress(app, 100.0, "runtime", "portable runtime not bundled");
        return Ok(None);
    }

    cleanup_stale_portable_runtime_dirs();
    let runtime_dir = create_portable_temp_dir()?;
    let _ = PORTABLE_RUNTIME_ROOT.set(runtime_dir.clone());
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

fn service_dir() -> PathBuf {
    host_root().join("service")
}

fn service_request_path() -> PathBuf {
    service_dir().join("diag_request.txt")
}

fn service_response_path() -> PathBuf {
    service_dir().join("diag_response.txt")
}

fn service_state_path() -> PathBuf {
    service_dir().join("diag_state.txt")
}

fn current_device_uid_cell() -> &'static Mutex<Option<String>> {
    CURRENT_DEVICE_UID.get_or_init(|| Mutex::new(None))
}

fn set_current_device_uid(uid: Option<String>) {
    if let Ok(mut slot) = current_device_uid_cell().lock() {
        *slot = uid;
    }
}

fn current_device_uid() -> Option<String> {
    current_device_uid_cell().lock().ok().and_then(|slot| slot.clone())
}

fn service_bridge_state_cell() -> &'static Mutex<ServiceBridgeState> {
    SERVICE_BRIDGE_STATE.get_or_init(|| {
        Mutex::new(ServiceBridgeState {
            id: "0".to_string(),
            command: "status".to_string(),
            ok: true,
            message: "GW Studio ready".to_string(),
            device_uid: "UNKNOWN".to_string(),
            programmer: "UNKNOWN".to_string(),
            probe_vendor: "UNKNOWN".to_string(),
            probe_id: "UNKNOWN".to_string(),
            cpu_id: "UNKNOWN".to_string(),
            target_voltage: "UNKNOWN".to_string(),
            mcu_profile: "UNKNOWN".to_string(),
            detected_firmware: "UNKNOWN".to_string(),
            external_flash: "UNKNOWN".to_string(),
            protection: "UNKNOWN".to_string(),
            filesystem: "UNKNOWN".to_string(),
            backup_ready: false,
            updated_at: timestamp_string(),
        })
    })
}

fn timestamp_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    format!("{}", now.as_secs())
}

fn service_state_text(state: &ServiceBridgeState) -> String {
    [
        format!("id={}", state.id),
        format!("command={}", state.command),
        format!("ok={}", state.ok),
        format!("message={}", state.message),
        format!("device_uid={}", state.device_uid),
        format!("programmer={}", state.programmer),
        format!("probe_vendor={}", state.probe_vendor),
        format!("probe_id={}", state.probe_id),
        format!("cpu_id={}", state.cpu_id),
        format!("target_voltage={}", state.target_voltage),
        format!("mcu_profile={}", state.mcu_profile),
        format!("detected_firmware={}", state.detected_firmware),
        format!("external_flash={}", state.external_flash),
        format!("protection={}", state.protection),
        format!("filesystem={}", state.filesystem),
        format!("backup_ready={}", state.backup_ready),
        format!("updated_at={}", state.updated_at),
    ]
    .join("\n")
}

fn service_bridge_snapshot() -> ServiceBridgeState {
    service_bridge_state_cell()
        .lock()
        .ok()
        .map(|state| state.clone())
        .unwrap_or_else(|| ServiceBridgeState {
            id: "0".to_string(),
            command: "status".to_string(),
            ok: true,
            message: "GW Studio ready".to_string(),
            device_uid: "UNKNOWN".to_string(),
            programmer: "UNKNOWN".to_string(),
            probe_vendor: "UNKNOWN".to_string(),
            probe_id: "UNKNOWN".to_string(),
            cpu_id: "UNKNOWN".to_string(),
            target_voltage: "UNKNOWN".to_string(),
            mcu_profile: "UNKNOWN".to_string(),
            detected_firmware: "UNKNOWN".to_string(),
            external_flash: "UNKNOWN".to_string(),
            protection: "UNKNOWN".to_string(),
            filesystem: "UNKNOWN".to_string(),
            backup_ready: false,
            updated_at: timestamp_string(),
        })
}

fn backup_ready_for_device(device_uid: &str) -> bool {
    let uid = device_uid.trim();
    if uid.is_empty() || uid.eq_ignore_ascii_case("UNKNOWN") {
        return false;
    }

    let dir = device_backups_dir(uid);
    let legacy_dir = legacy_device_backups_dir(uid);
    let mcu = newest_matching_file(&dir, "bank1")
        .or_else(|| newest_matching_file(&dir, "internal_flash_bank1_backup_"))
        .or_else(|| newest_matching_file(&dir, "internal_flash_backup_"))
        .or_else(|| newest_matching_file(&legacy_dir, "bank1"))
        .or_else(|| newest_matching_file(&legacy_dir, "internal_flash_bank1_backup_"))
        .or_else(|| newest_matching_file(&legacy_dir, "internal_flash_backup_"));
    let spi = newest_named_backup_file(&dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"])
        .or_else(|| newest_matching_file(&dir, "spi_"))
        .or_else(|| newest_matching_file(&dir, "flash_backup_"))
        .or_else(|| newest_named_backup_file(&legacy_dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"]))
        .or_else(|| newest_matching_file(&legacy_dir, "spi_"))
        .or_else(|| newest_matching_file(&legacy_dir, "flash_backup_"));

    mcu.is_some() && spi.is_some()
}

fn write_service_bridge_snapshot_file() -> Result<(), String> {
    fs::create_dir_all(service_dir())
        .map_err(|error| format!("failed to create service dir: {error}"))?;
    let snapshot = service_bridge_snapshot();
    let path = service_state_path();
    fs::write(&path, service_state_text(&snapshot))
        .map_err(|error| format!("failed to write service snapshot: {error}"))?;
    Ok(())
}

fn update_service_bridge_state(message: &str, device_info: Option<&DeviceInfo>) {
    if let Ok(mut state) = service_bridge_state_cell().lock() {
        state.message = message.to_string();
        state.updated_at = timestamp_string();
        if let Some(info) = device_info {
            state.device_uid = info.device_uid.clone();
            state.programmer = info.programmer.clone();
            state.probe_vendor = info.probe_vendor.clone();
            state.probe_id = info.probe_id.clone();
            state.cpu_id = info.cpu_id.clone();
            state.target_voltage = info.target_voltage.clone();
            state.mcu_profile = info.mcu_profile.clone();
            state.detected_firmware = info.detected_firmware.clone();
            state.external_flash = info.external_flash.clone();
            state.protection = info.protection.clone();
            state.filesystem = info.filesystem.clone();
            state.backup_ready = backup_ready_for_device(&info.device_uid);
        }
    }
    let _ = write_service_bridge_snapshot_file();
}

fn parse_service_bridge_request(raw: &str) -> Option<ServiceBridgeRequest> {
    let mut id = String::new();
    let mut command = String::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            match key.trim() {
                "id" => id = value.trim().to_string(),
                "command" => command = value.trim().to_string(),
                _ => {}
            }
        }
    }

    if id.is_empty() || command.is_empty() {
        None
    } else {
        Some(ServiceBridgeRequest { id, command })
    }
}

fn write_service_bridge_response(response: &ServiceBridgeResponse) -> Result<(), String> {
    fs::create_dir_all(service_dir()).map_err(|error| format!("failed to create service dir: {error}"))?;
    let path = service_response_path();
    let response_text = [
        format!("id={}", response.id),
        format!("command={}", response.command),
        format!("ok={}", response.ok),
        format!("message={}", response.message),
        service_state_text(&response.state),
    ]
    .join("\n");
    fs::write(&path, response_text).map_err(|error| format!("failed to write service response: {error}"))?;
    Ok(())
}

fn handle_service_bridge_request(request: ServiceBridgeRequest) -> ServiceBridgeResponse {
    let state = service_bridge_snapshot();
    let command = request.command.trim().to_ascii_lowercase();
    match command.as_str() {
        "status" | "snapshot" | "ping" => ServiceBridgeResponse {
            id: request.id,
            command: request.command,
            ok: true,
            message: "status snapshot".to_string(),
            state,
        },
        _ => ServiceBridgeResponse {
            id: request.id,
            command: request.command,
            ok: false,
            message: "unknown command".to_string(),
            state,
        },
    }
}

fn start_service_bridge_listener() {
    let _ = fs::create_dir_all(service_dir());
    let _ = write_service_bridge_snapshot_file();
    thread::spawn(|| loop {
        let request_path = service_request_path();
        if let Ok(raw_request) = fs::read_to_string(&request_path) {
            if let Some(request) = parse_service_bridge_request(&raw_request) {
                let response = handle_service_bridge_request(request);
                let _ = write_service_bridge_response(&response);
                let _ = fs::remove_file(&request_path);
            }
        }
        thread::sleep(Duration::from_millis(250));
    });
}

fn backups_dir() -> PathBuf {
    workspace_root().join("backups")
}

fn device_backups_dir(uid: &str) -> PathBuf {
    backups_dir().join(uid)
}

fn stock_firmware_dir() -> PathBuf {
    host_root().join("StockFirmware")
}

fn legacy_device_backups_dir(uid: &str) -> PathBuf {
    workspace_root().join("devices").join(uid).join("backups")
}

fn required_active_backups_dir() -> Result<PathBuf, String> {
    current_device_uid()
        .filter(|uid| !uid.trim().is_empty() && uid != "UNKNOWN")
        .map(|uid| device_backups_dir(&uid))
        .ok_or_else(|| "MCU UID is unknown; run Read Device Info before reading backups".to_string())
}

fn newest_matching_file(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(prefix) && name.ends_with(".bin"))
                .unwrap_or(false)
        })
        .collect();

    matches.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    matches.pop()
}

fn matching_files(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(prefix) && name.ends_with(".bin"))
                .unwrap_or(false)
        })
        .collect();

    matches.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    matches.reverse();
    matches
}

fn newest_named_backup_file(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.is_file())
        .max_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok())
}

fn newest_restore_bank1_file(dir: &Path, legacy_dir: &Path) -> Option<PathBuf> {
    newest_matching_file(dir, "internal_flash_bank1_backup_")
        .or_else(|| newest_matching_file(dir, "internal_flash_backup_"))
        .or_else(|| newest_matching_file(dir, "bank1"))
        .or_else(|| newest_matching_file(legacy_dir, "internal_flash_bank1_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "internal_flash_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "bank1"))
}

fn newest_restore_bank2_file(dir: &Path, legacy_dir: &Path) -> Option<PathBuf> {
    newest_matching_file(dir, "internal_flash_bank2_backup_")
        .or_else(|| newest_matching_file(dir, "bank2"))
        .or_else(|| newest_matching_file(legacy_dir, "internal_flash_bank2_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "bank2"))
}

fn newest_restore_spi_file(dir: &Path, legacy_dir: &Path) -> Option<PathBuf> {
    newest_matching_file(dir, "flash_backup_")
        .or_else(|| newest_matching_file(dir, "spi_backup_"))
        .or_else(|| newest_named_backup_file(dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"]))
        .or_else(|| newest_matching_file(dir, "spi_"))
        .or_else(|| newest_matching_file(legacy_dir, "flash_backup_"))
        .or_else(|| newest_matching_file(legacy_dir, "spi_backup_"))
        .or_else(|| newest_named_backup_file(legacy_dir, &["spi_m.bin", "spim.bin", "spi_z.bin", "spiz.bin"]))
        .or_else(|| newest_matching_file(legacy_dir, "spi_"))
}

fn newest_valid_stock_bank1_file(dir: &Path, legacy_dir: &Path, profile_code: &str) -> Option<PathBuf> {
    [dir, legacy_dir]
        .into_iter()
        .flat_map(|candidate_dir| matching_files(candidate_dir, "stock_bank1"))
        .find(|path| validate_stock_bank1_file(path, profile_code).is_ok())
}

fn newest_valid_stock_spi_file(dir: &Path, legacy_dir: &Path, profile_code: &str) -> Option<PathBuf> {
    [dir, legacy_dir]
        .into_iter()
        .flat_map(|candidate_dir| matching_files(candidate_dir, "stock_spi"))
        .find(|path| validate_stock_spi_file(path, profile_code).is_ok())
}

fn bundles_dir() -> PathBuf {
    workspace_root().join("bundles")
}

fn build_workspaces_dir() -> PathBuf {
    workspace_root().join("build_workspaces")
}

fn rom_imports_dir() -> PathBuf {
    workspace_root().join("rom_imports")
}

fn thumbnails_dir() -> PathBuf {
    host_root().join("content").join("image")
}

fn msx_bios_store_dir() -> PathBuf {
    host_root().join("msx_bios")
}

fn coleco_bios_store_dir() -> PathBuf {
    host_root().join("coleco_bios")
}

fn legacy_thumbnails_dir() -> PathBuf {
    workspace_root().join("thumbnails")
}

const COLECO_BIOS_FILE_NAME: &str = "coleco.rom";
const COLECO_BIOS_SIZE: usize = 8192;
const COLECO_BIOS_SHA1: &str = "2f625916c6458379379e61c91ecab3439624d8bf";

const REQUIRED_MSX_BIOS_FILES: &[&str] = &[
    "MSX2P.rom",
    "MSX2PEXT.rom",
    "MSX2PMUS.rom",
    "MSX2.rom",
    "MSX2EXT.rom",
    "MSX.rom",
    "PANASONICDISK.rom",
];

fn sha1_hex(bytes: &[u8]) -> String {
    let mut h0: u32 = 0x6745_2301;
    let mut h1: u32 = 0xefcd_ab89;
    let mut h2: u32 = 0x98ba_dcfe;
    let mut h3: u32 = 0x1032_5476;
    let mut h4: u32 = 0xc3d2_e1f0;

    let bit_len = (bytes.len() as u64) * 8;
    let mut message = Vec::with_capacity(bytes.len() + 72);
    message.extend_from_slice(bytes);
    message.push(0x80);
    while (message.len() % 64) != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut words = [0_u32; 80];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..80 {
            words[index] = (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                .rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for (index, word) in words.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999),
                20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    format!("{h0:08x}{h1:08x}{h2:08x}{h3:08x}{h4:08x}")
}

fn canonical_msx_bios_name(file_name: &str) -> Option<&'static str> {
    let lower = file_name.to_ascii_lowercase();
    REQUIRED_MSX_BIOS_FILES
        .iter()
        .copied()
        .find(|required| required.to_ascii_lowercase() == lower)
}

fn msx_bios_status() -> MsxBiosStatus {
    let dir = msx_bios_store_dir();
    let missing = REQUIRED_MSX_BIOS_FILES
        .iter()
        .filter(|name| !dir.join(name).is_file())
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    MsxBiosStatus {
        ready: missing.is_empty(),
        dir: dir.to_string_lossy().to_string(),
        missing,
    }
}

fn save_msx_bios_file(file_name: &str, bytes: &[u8]) -> Result<bool, String> {
    let Some(canonical_name) = canonical_msx_bios_name(file_name) else {
        return Ok(false);
    };
    let dir = msx_bios_store_dir();
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create MSX BIOS dir {}: {error}", dir.display()))?;
    fs::write(dir.join(canonical_name), bytes)
        .map_err(|error| format!("failed to save MSX BIOS {canonical_name}: {error}"))?;
    Ok(true)
}

fn copy_msx_bios_to_workspace(workspace_dir: &Path) -> Result<(), String> {
    let status = msx_bios_status();
    if !status.ready {
        return Err(format!(
            "MSX BIOS files are required. Drop all BIOS files first. Missing: {}",
            status.missing.join(", ")
        ));
    }
    let source_dir = msx_bios_store_dir();
    let dest_dir = workspace_dir.join("roms").join("msx_bios");
    fs::create_dir_all(&dest_dir)
        .map_err(|error| format!("failed to create Retro-Go MSX BIOS dir {}: {error}", dest_dir.display()))?;
    for name in REQUIRED_MSX_BIOS_FILES {
        fs::copy(source_dir.join(name), dest_dir.join(name))
            .map_err(|error| format!("failed to copy MSX BIOS {name}: {error}"))?;
    }
    Ok(())
}

fn coleco_bios_status() -> MsxBiosStatus {
    let dir = coleco_bios_store_dir();
    let path = dir.join(COLECO_BIOS_FILE_NAME);
    let missing = match fs::read(&path) {
        Ok(bytes) if validate_coleco_bios(&bytes).is_ok() => Vec::new(),
        _ => vec![format!(
            "{} ({} bytes, SHA-1 {})",
            COLECO_BIOS_FILE_NAME, COLECO_BIOS_SIZE, COLECO_BIOS_SHA1
        )],
    };
    MsxBiosStatus {
        ready: missing.is_empty(),
        dir: dir.to_string_lossy().to_string(),
        missing,
    }
}

fn validate_coleco_bios(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() != COLECO_BIOS_SIZE {
        return Err(format!(
            "invalid ColecoVision BIOS size: got {} bytes, expected {} bytes",
            bytes.len(),
            COLECO_BIOS_SIZE
        ));
    }
    let actual_sha1 = sha1_hex(bytes);
    if actual_sha1 != COLECO_BIOS_SHA1 {
        return Err(format!(
            "invalid ColecoVision BIOS SHA-1: got {}, expected {}",
            actual_sha1, COLECO_BIOS_SHA1
        ));
    }
    Ok(())
}

fn save_coleco_bios_file(file_name: &str, bytes: &[u8]) -> Result<bool, String> {
    validate_coleco_bios(bytes).map_err(|error| format!("{file_name}: {error}"))?;
    let dir = coleco_bios_store_dir();
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create ColecoVision BIOS dir {}: {error}", dir.display()))?;
    fs::write(dir.join(COLECO_BIOS_FILE_NAME), bytes)
        .map_err(|error| format!("failed to save ColecoVision BIOS {}: {error}", COLECO_BIOS_FILE_NAME))?;
    Ok(true)
}

fn write_coleco_bios_header(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut header = String::from(
        "// Generated by GW Studio from user-provided coleco.rom.\n\
         const unsigned char ColecoVision_BIOS[] __attribute__((section (\".extflash_data\"))) = {\n",
    );
    for (index, byte) in bytes.iter().enumerate() {
        if index % 16 == 0 {
            header.push_str("    ");
        }
        header.push_str(&format!("0x{byte:02x},"));
        if index % 16 == 15 {
            header.push('\n');
        } else {
            header.push(' ');
        }
    }
    if bytes.len() % 16 != 0 {
        header.push('\n');
    }
    header.push_str("};\n");
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create ColecoVision BIOS header dir {}: {error}", parent.display()))?;
    }
    fs::write(dest, header)
        .map_err(|error| format!("failed to write ColecoVision BIOS header {}: {error}", dest.display()))
}

fn copy_coleco_bios_to_workspace(workspace_dir: &Path) -> Result<(), String> {
    let source = coleco_bios_store_dir().join(COLECO_BIOS_FILE_NAME);
    let bytes = fs::read(&source).map_err(|error| {
        format!(
            "ColecoVision BIOS is required. Drop {} first. Failed to read {}: {error}",
            COLECO_BIOS_FILE_NAME,
            source.display()
        )
    })?;
    validate_coleco_bios(&bytes)?;
    let dest = workspace_dir
        .join("retro-go-stm32")
        .join("smsplusgx-go")
        .join("components")
        .join("smsplus")
        .join("coleco_bios.h");
    write_coleco_bios_header(&bytes, &dest)
}

fn sanitize_thumbnail_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_' | '(' | ')' | ',' | '.' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn thumbnail_cache_path(emulator: &str, title: &str) -> PathBuf {
    thumbnails_dir()
        .join(sanitize_thumbnail_part(emulator))
        .join(format!("{}.png", sanitize_thumbnail_part(title)))
}

fn legacy_thumbnail_cache_path(emulator: &str, title: &str) -> PathBuf {
    legacy_thumbnails_dir()
        .join(sanitize_thumbnail_part(emulator))
        .join(format!("{}.png", sanitize_thumbnail_part(title)))
}

fn normalize_thumbnail_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let source =
        image::load_from_memory(bytes).map_err(|error| format!("failed to decode image: {error}"))?;
    let mut cursor = Cursor::new(Vec::new());
    source
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| format!("failed to encode thumbnail png: {error}"))?;
    Ok(cursor.into_inner())
}

fn rom_extensions_for_emulator(emulator: &str) -> &'static [&'static str] {
    match emulator {
        "nes" => &[".nes", ".fds", ".nsf"],
        "gb" => &[".gb"],
        "gbc" => &[".gbc"],
        "sms" => &[".sms"],
        "gg" => &[".gg"],
        "pce" => &[".pce"],
        "col" => &[".col"],
        "gw" => &[".gw"],
        "md" => &[".md", ".bin", ".gen"],
        "sg" => &[".sg"],
        "msx" => &[".rom", ".mx1", ".mx2", ".dsk"],
        "wsv" => &[".sv", ".bin"],
        "a7800" => &[".a78", ".bin"],
        "tama" => &[".b"],
        _ => &[],
    }
}

fn emulator_ids() -> &'static [&'static str] {
    &[
        "nes", "gb", "gbc", "sms", "gg", "pce", "col", "gw", "md", "sg", "msx", "wsv",
        "a7800", "tama",
    ]
}

fn ambiguous_rom_extension(extension: &str) -> bool {
    matches!(extension, ".bin" | ".dsk" | ".sfc")
}

fn file_extension_lower(file_name: &str) -> Option<String> {
    std::path::Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_ascii_lowercase()))
}

fn emulator_for_file_name(file_name: &str) -> Option<&'static str> {
    let lower_name = file_name.to_ascii_lowercase();
    let extension = file_extension_lower(file_name)?;

    if ambiguous_rom_extension(&extension) {
        return None;
    }

    for emulator in emulator_ids() {
        if rom_extensions_for_emulator(emulator)
            .iter()
            .any(|ext| lower_name.ends_with(ext))
        {
            return Some(emulator);
        }
    }
    None
}

fn is_ambiguous_rom_file_name(file_name: &str) -> bool {
    file_extension_lower(file_name)
        .as_deref()
        .map(ambiguous_rom_extension)
        .unwrap_or(false)
}

fn is_supported_import_name(file_name: &str) -> bool {
    let lower_name = file_name.to_ascii_lowercase();
    emulator_for_file_name(file_name).is_some()
        || is_ambiguous_rom_file_name(file_name)
        || lower_name.ends_with(".zip")
        || lower_name.ends_with(".7z")
}

fn import_auto_from_named_bytes(
    file_name: &str,
    bytes: Vec<u8>,
    entries: &mut Vec<AutoRomImportEntry>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let lower_name = file_name.to_ascii_lowercase();
    let source_label = std::path::Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("import");

    if let Some(emulator) = emulator_for_file_name(file_name) {
        let dest_path = write_imported_rom(emulator, source_label, file_name, &bytes)?;
        let title = std::path::Path::new(file_name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(file_name)
            .to_string();
        entries.push(AutoRomImportEntry {
            emulator: emulator.to_string(),
            title,
            path: dest_path.to_string_lossy().to_string(),
            size_bytes: bytes.len() as u64,
        });
        return Ok(());
    }

    if is_ambiguous_rom_file_name(file_name) {
        let message = if file_extension_lower(file_name).as_deref() == Some(".sfc") {
            "SFC ROM skipped: SMW/Zelda3 direct loaders were removed from this build"
        } else {
            "Ambiguous ROM skipped. Add it with the + button for the target emulator."
        };
        warnings.push(format!("{message}: {file_name}"));
        return Ok(());
    }

    if lower_name.ends_with(".zip") {
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|error| format!("failed to open zip archive {}: {error}", file_name))?;

        for index in 0..archive.len() {
            let mut item = archive
                .by_index(index)
                .map_err(|error| format!("failed to read zip entry in {}: {error}", file_name))?;
            if item.is_dir() {
                continue;
            }
            let entry_name = item.name().replace('\\', "/");
            let Some(emulator) = emulator_for_file_name(&entry_name) else {
                if is_ambiguous_rom_file_name(&entry_name) {
                    let message = if file_extension_lower(&entry_name).as_deref() == Some(".sfc") {
                        "SFC ROM skipped: SMW/Zelda3 direct loaders were removed from this build"
                    } else {
                        "Ambiguous ROM skipped. Add it with the + button for the target emulator."
                    };
                    warnings.push(format!("{message} In {file_name}: {entry_name}"));
                }
                continue;
            };
            let base_name = std::path::Path::new(&entry_name)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(&entry_name)
                .to_string();
            let mut content = Vec::new();
            item.read_to_end(&mut content)
                .map_err(|error| format!("failed to extract zip entry {entry_name}: {error}"))?;
            let dest_path = write_imported_rom(emulator, source_label, &base_name, &content)?;
            let title = std::path::Path::new(&base_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or(&base_name)
                .to_string();
            entries.push(AutoRomImportEntry {
                emulator: emulator.to_string(),
                title,
                path: dest_path.to_string_lossy().to_string(),
                size_bytes: content.len() as u64,
            });
        }
        return Ok(());
    }

    if lower_name.ends_with(".7z") {
        warnings.push(format!(
            "7z archive is not supported yet: {}. Repack to ZIP or extract manually.",
            file_name
        ));
        return Ok(());
    }

    warnings.push(format!("Unsupported file skipped: {}", file_name));
    Ok(())
}

fn sanitize_file_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_' | '(' | ')' | ',' | '.' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn copy_if_exists(source: &PathBuf, destination: &PathBuf) -> Result<bool, String> {
    if !source.exists() {
        return Ok(false);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("failed to create parent dir: {error}"))?;
    }
    fs::copy(source, destination).map_err(|error| format!("failed to copy file: {error}"))?;
    Ok(true)
}

fn bundle_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    format!("{}", now.as_secs())
}

fn firmware_profile_code(value: &str) -> &'static str {
    match value.trim().to_ascii_uppercase().as_str() {
        "Z" => "z",
        _ => "m",
    }
}

fn firmware_output_name(part: &str, profile_code: &str) -> String {
    match part {
        "spi" => format!("spi_{profile_code}.bin"),
        _ => format!("{part}{profile_code}.bin"),
    }
}

fn stock_firmware_output_name(part: &str, profile_code: &str) -> String {
    format!("stock_{}", firmware_output_name(part, profile_code))
}

const RETRO_GO_FORK_DIR_NAME: &str = "game-and-watch-retro-go-sylverb";

fn local_retro_go_candidate_from(start: PathBuf) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(RETRO_GO_FORK_DIR_NAME);
        if candidate.join("Makefile").is_file() {
            return Some(candidate);
        }
    }
    None
}

fn locate_retro_go_repo() -> Option<PathBuf> {
    [
        host_root().join(RETRO_GO_FORK_DIR_NAME),
        portable_source_dir().join(RETRO_GO_FORK_DIR_NAME),
    ]
    .into_iter()
    .find(|path| path.join("Makefile").is_file())
    .or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .and_then(local_retro_go_candidate_from)
    })
    .or_else(|| std::env::current_dir().ok().and_then(local_retro_go_candidate_from))
    .or_else(|| local_retro_go_candidate_from(PathBuf::from(env!("CARGO_MANIFEST_DIR"))))
}

fn explicit_stock_file(path: Option<&str>) -> Result<PathBuf, String> {
    let raw_path = path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "UI did not provide stock firmware path".to_string())?;
    let selected = PathBuf::from(raw_path);
    if !selected.is_file() {
        return Err(format!("UI stock firmware path is not a file: {}", selected.display()));
    }

    let stock_dir = stock_firmware_dir();
    fs::create_dir_all(&stock_dir)
        .map_err(|error| format!("failed to create StockFirmware dir: {error}"))?;
    let stock_dir = stock_dir
        .canonicalize()
        .map_err(|error| format!("failed to resolve StockFirmware dir: {error}"))?;
    let selected = selected
        .canonicalize()
        .map_err(|error| format!("failed to resolve UI stock firmware path: {error}"))?;
    if !selected.starts_with(&stock_dir) {
        return Err(format!(
            "UI stock firmware path must be inside StockFirmware folder {}: {}",
            stock_dir.display(),
            selected.display()
        ));
    }
    Ok(selected)
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

fn patch_game_watch_patch_workspace_for_modern_gcc(
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

fn locate_python_exe() -> PathBuf {
    [
        tool_candidate(&["python", "python.exe"]),
        tool_candidate(&["python", "python3.exe"]),
        tool_candidate(&["python", "Scripts", "python.exe"]),
        host_root().join("python").join("python.exe"),
    ]
    .into_iter()
    .find(|path| path.exists())
    .unwrap_or_else(|| PathBuf::from("python"))
}

fn copy_file_prefix(source: &Path, destination: &Path, max_bytes: u64) -> Result<(), String> {
    let input = fs::File::open(source)
        .map_err(|error| format!("failed to open {}: {error}", source.display()))?;
    let mut limited = input.take(max_bytes);
    let mut output = fs::File::create(destination)
        .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
    std::io::copy(&mut limited, &mut output)
        .map_err(|error| format!("failed to copy {} prefix: {error}", source.display()))?;
    Ok(())
}

fn compose_spi_image(stock_spi_path: &Path, retro_go_extflash_path: &Path, destination: &Path, offset_bytes: u64) -> Result<(), String> {
    copy_file_prefix(stock_spi_path, destination, offset_bytes)?;
    if !retro_go_extflash_path.is_file() {
        return Ok(());
    }
    let mut output = fs::OpenOptions::new()
        .write(true)
        .open(destination)
        .map_err(|error| format!("failed to open {} for SPI composition: {error}", destination.display()))?;
    output
        .seek(SeekFrom::Start(offset_bytes))
        .map_err(|error| format!("failed to seek {}: {error}", destination.display()))?;
    let mut payload = fs::File::open(retro_go_extflash_path)
        .map_err(|error| format!("failed to open {}: {error}", retro_go_extflash_path.display()))?;
    std::io::copy(&mut payload, &mut output)
        .map_err(|error| format!("failed to append Retro-Go fork SPI payload: {error}"))?;
    Ok(())
}

fn read_file_prefix(source: &Path, max_bytes: usize) -> Result<Vec<u8>, String> {
    let mut input = fs::File::open(source)
        .map_err(|error| format!("failed to open {}: {error}", source.display()))?;
    let mut data = vec![0_u8; max_bytes];
    input
        .read_exact(&mut data)
        .map_err(|error| format!("failed to read {} prefix: {error}", source.display()))?;
    Ok(data)
}

fn validate_stock_bank1_file(stock_bank1_path: &Path, profile_code: &str) -> Result<(), String> {
    let bank1 = read_file_prefix(stock_bank1_path, 128 * 1024)?;
    let bank1_crc = crc32fast::hash(&bank1);
    let expected_bank1_crc = if profile_code == "z" { 0x3420bccf } else { 0xefaaefe6 };
    if bank1_crc != expected_bank1_crc {
        return Err(format!(
            "Selected Bank1 is not a valid {} stock firmware for dualboot patch: {} (CRC32 {:08X}, expected {:08X})",
            profile_code.to_ascii_uppercase(),
            stock_bank1_path.display(),
            bank1_crc,
            expected_bank1_crc
        ));
    }
    Ok(())
}

fn detect_stock_bank1_profile(stock_bank1_path: &Path) -> Result<&'static str, String> {
    let bank1 = read_file_prefix(stock_bank1_path, 128 * 1024)?;
    match crc32fast::hash(&bank1) {
        0xefaaefe6 => Ok("m"),
        0x3420bccf => Ok("z"),
        crc => Err(format!(
            "Selected Bank1 is not a known original Mario/Zelda stock firmware: {} (CRC32 {:08X})",
            stock_bank1_path.display(),
            crc
        )),
    }
}

fn validate_stock_spi_file(stock_spi_path: &Path, profile_code: &str) -> Result<(), String> {
    let stock_spi = fs::read(stock_spi_path)
        .map_err(|error| format!("failed to read stock SPI {}: {error}", stock_spi_path.display()))?;
    let (spi_region, expected_spi_crc) = if profile_code == "z" {
        let start = 0x20000_usize;
        let end = 0x3254A0_usize;
        if stock_spi.len() < end {
            return Err(format!(
                "Selected SPI stock backup is too small for Zelda dualboot patch: {} bytes, need at least {} bytes",
                stock_spi.len(),
                end
            ));
        }
        (&stock_spi[start..end], 0x07a478d4_u32)
    } else {
        let strip_tail = 8192_usize;
        let min_len = (1024 * 1024) as usize;
        if stock_spi.len() < min_len {
            return Err(format!(
                "Selected SPI stock backup is too small for Mario dualboot patch: {} bytes, need at least {} bytes",
                stock_spi.len(),
                min_len
            ));
        }
        (&stock_spi[..min_len - strip_tail], 0x5f40d6bb_u32)
    };
    let spi_crc = crc32fast::hash(spi_region);
    if spi_crc != expected_spi_crc {
        return Err(format!(
            "Selected SPI is not a valid {} stock firmware for dualboot patch: {} (CRC32 {:08X}, expected {:08X})",
            profile_code.to_ascii_uppercase(),
            stock_spi_path.display(),
            spi_crc,
            expected_spi_crc
        ));
    }

    Ok(())
}

fn detect_stock_spi_profile(stock_spi_path: &Path) -> Result<&'static str, String> {
    let stock_spi = fs::read(stock_spi_path)
        .map_err(|error| format!("failed to read stock SPI {}: {error}", stock_spi_path.display()))?;

    if stock_spi.len() >= 1024 * 1024 {
        let mario_region = &stock_spi[..(1024 * 1024) - 8192];
        if crc32fast::hash(mario_region) == 0x5f40d6bb {
            return Ok("m");
        }
    }

    let zelda_end = 0x3254A0_usize;
    if stock_spi.len() >= zelda_end {
        let zelda_region = &stock_spi[0x20000..zelda_end];
        if crc32fast::hash(zelda_region) == 0x07a478d4 {
            return Ok("z");
        }
    }

    Err(format!(
        "Selected SPI is not a known original Mario/Zelda stock firmware: {}",
        stock_spi_path.display()
    ))
}

fn validate_patch_stock_inputs(stock_bank1_path: &Path, stock_spi_path: &Path, profile_code: &str) -> Result<(), String> {
    validate_stock_bank1_file(stock_bank1_path, profile_code)?;
    validate_stock_spi_file(stock_spi_path, profile_code)?;
    Ok(())
}

fn run_game_watch_patch_build(
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

fn retro_go_emulator_dir(emulator: &str) -> Option<&'static str> {
    match emulator {
        "nes" => Some("nes"),
        "gb" => Some("gb"),
        "gbc" => Some("gb"),
        "sms" => Some("sms"),
        "gg" => Some("gg"),
        "pce" => Some("pce"),
        "col" => Some("col"),
        "gw" => Some("gw"),
        "md" => Some("md"),
        "sg" => Some("sg"),
        "msx" => Some("msx"),
        "wsv" => Some("wsv"),
        "a7800" => Some("a7800"),
        "tama" => Some("tama"),
        _ => None,
    }
}

fn retro_go_workspace_rom_name(emulator: &str, fallback_name: &str) -> String {
    match emulator {
        _ => sanitize_file_part(fallback_name),
    }
}

fn ensure_clean_dir(path: &PathBuf) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| format!("failed to clear dir {}: {error}", path.display()))?;
    }
    fs::create_dir_all(path).map_err(|error| format!("failed to create dir {}: {error}", path.display()))?;
    Ok(())
}

fn copy_dir_filtered(source: &PathBuf, destination: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("failed to create dir {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source).map_err(|error| format!("failed to read dir {}: {error}", source.display()))? {
        let entry = entry.map_err(|error| format!("failed to read dir entry: {error}"))?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == ".git" || name_str == "build" {
            continue;
        }
        let dest_path = destination.join(&name);
        if path.is_dir() {
            copy_dir_filtered(&path, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create dir {}: {error}", parent.display()))?;
            }
            fs::copy(&path, &dest_path)
                .map_err(|error| format!("failed to copy {} -> {}: {error}", path.display(), dest_path.display()))?;
        }
    }
    Ok(())
}

fn patch_retro_go_workspace_for_windows(workspace_dir: &PathBuf, coverflow_enabled: bool) -> Result<(), String> {
    let makefile_common = workspace_dir.join("Makefile.common");
    let original = fs::read_to_string(&makefile_common)
        .map_err(|error| format!("failed to read {}: {error}", makefile_common.display()))?;
    let git_bin = locate_git_bin_dir().ok_or_else(|| "Git bash/sh not found".to_string())?;
    let python_exe = locate_python_exe();
    let python3_exe_path = to_bash_path_for(&python_exe, &git_bin);
    let python3_make_command = format!(
        "\"{}\" -c \"import sys,runpy; from pathlib import Path; script=sys.argv.pop(1); sys.path.insert(0,str(Path(script).resolve().parent)); sys.path.insert(0,'.'); runpy.run_path(script, run_name='__main__')\"",
        python3_exe_path
    );
    let normalized = original
        .replace("\r\n", "\n")
        .replace("PYTHON3 ?= /usr/bin/env python3", &format!("PYTHON3 ?= {python3_make_command}"))
        .replace(
            "\t$(V)wget -q $(SDK_URL)/$(SDK_VERSION)/$@ -P $(dir $@)",
            "\t$(V)mkdir -p $(dir $@) && curl -L --silent --fail $(SDK_URL)/$(SDK_VERSION)/$@ -o $@",
        )
        .replace("\t$(V)./scripts/", "\t$(V)bash ./scripts/")
        .replace("\t$(V)/bin/sh -c true", "\t$(V)sh -c true");
    let mut patched_lines = Vec::new();
    patched_lines.push("SHELL := bash.exe".to_string());
    let source_prereq_patch = r#"# GW Studio patch: keep same-named emulator sources from colliding through global vpath.
define gw_studio_c_obj_rule
$$(BUILD_DIR)/$(1)/$(notdir $(patsubst %.c,%.o,$(2))): $(2) Makefile.common Makefile $$(SDK_HEADERS) $$(BUILD_DIR)/config.h | $$(BUILD_DIR)
	$$(V)$$(ECHO) [ CC $(3) ] $$(notdir $$<)
	$$(V)$$(CC) -c $$(CFLAGS) $$($(4)) $(5) -Wa,-a,-ad,-alms=$$(BUILD_DIR)/$(1)/$$(notdir $$(<:.c=.lst)) $$< -o $$@

endef
$(eval $(foreach obj,$(NES_C_SOURCES),$(call gw_studio_c_obj_rule,nes,$(obj),nes,NES_C_INCLUDES,)))
$(eval $(foreach obj,$(NES_FCEU_C_SOURCES),$(call gw_studio_c_obj_rule,nes_fceu,$(obj),nes-fceu,NES_FCEU_C_INCLUDES,-Wno-sequence-point -Wno-parentheses)))
$(eval $(foreach obj,$(GNUBOY_C_SOURCES),$(call gw_studio_c_obj_rule,gnuboy,$(obj),gb,GNUBOY_C_INCLUDES,)))
$(eval $(foreach obj,$(SMSPLUSGX_C_SOURCES),$(call gw_studio_c_obj_rule,smsplusgx,$(obj),sms,SMSPLUSGX_C_INCLUDES,)))
$(eval $(foreach obj,$(PCE_C_SOURCES),$(call gw_studio_c_obj_rule,pce,$(obj),pce,PCE_C_INCLUDES,)))
$(eval $(foreach obj,$(GW_C_SOURCES),$(call gw_studio_c_obj_rule,gw,$(obj),gw,GW_C_INCLUDES,)))
$(eval $(foreach obj,$(MSX_C_SOURCES),$(call gw_studio_c_obj_rule,msx,$(obj),msx,MSX_C_INCLUDES,)))
$(eval $(foreach obj,$(WSV_C_SOURCES),$(call gw_studio_c_obj_rule,wsv,$(obj),wsv,WSV_C_INCLUDES,)))
$(eval $(foreach obj,$(MD_C_SOURCES),$(call gw_studio_c_obj_rule,md,$(obj),md,MD_C_INCLUDES,)))
$(eval $(foreach obj,$(A7800_C_SOURCES),$(call gw_studio_c_obj_rule,a7800,$(obj),a7800,A7800_C_INCLUDES,)))
$(eval $(foreach obj,$(TAMA_C_SOURCES),$(call gw_studio_c_obj_rule,tama,$(obj),tama,TAMA_C_INCLUDES,)))
"#;

    let mut lines = normalized.lines().peekable();
    while let Some(line) = lines.next() {
        if line == "# generate all object prerequisite rules" {
            patched_lines.extend(source_prereq_patch.lines().map(|line| line.to_string()));
        }
        if line == "$(BUILD_DIR):" {
            patched_lines.push(line.to_string());
            while let Some(next_line) = lines.peek() {
                if next_line.starts_with('\t') || next_line.trim().is_empty() {
                    lines.next();
                    continue;
                }
                break;
            }
            patched_lines.push("\t$(V)mkdir -p $@/core $@/nes $@/nes_fceu $@/gnuboy $@/smsplusgx $@/pce $@/gw $@/msx $@/wsv $@/md $@/a7800 $@/tama".to_string());
            continue;
        }
        patched_lines.push(line.to_string());
    }

    fs::write(&makefile_common, patched_lines.join("\n"))
        .map_err(|error| format!("failed to write {}: {error}", makefile_common.display()))?;

    if coverflow_enabled {
        let rg_main_c = workspace_dir.join("Core").join("Src").join("retro-go").join("rg_main.c");
        let rg_main_original = fs::read_to_string(&rg_main_c)
            .map_err(|error| format!("failed to read {}: {error}", rg_main_c.display()))?;
        let rg_main_patched = rg_main_original
            .replace("\r\n", "\n")
            .replace("    // gui.show_cover = odroid_settings_int32_get(KEY_SHOW_COVER, 1);", "    gui.show_cover = 1;");
        if rg_main_patched != rg_main_original {
            fs::write(&rg_main_c, rg_main_patched)
                .map_err(|error| format!("failed to write {}: {error}", rg_main_c.display()))?;
        }
    }

    let rg_emulators_c = workspace_dir.join("Core").join("Src").join("retro-go").join("rg_emulators.c");
    if rg_emulators_c.exists() {
        let rg_emulators_original = fs::read_to_string(&rg_emulators_c)
            .map_err(|error| format!("failed to read {}: {error}", rg_emulators_c.display()))?;
        let rg_emulators_patched = rg_emulators_original
            .replace("\r\n", "\n")
            .replace("#include \"main_amstrad.h\"\n", "");
        if rg_emulators_patched != rg_emulators_original {
            fs::write(&rg_emulators_c, rg_emulators_patched)
                .map_err(|error| format!("failed to write {}: {error}", rg_emulators_c.display()))?;
        }
    }

    let extflash_size_script = workspace_dir.join("scripts").join("extflash_size.sh");
let extflash_size_script_patched = r#"#!/bin/bash
# Usage: ./extflash_size.sh app.elf

export LC_ALL=C

if [[ "${GCC_PATH}" != "" ]]; then
	DEFAULT_OBJDUMP=${GCC_PATH}/arm-none-eabi-objdump
else
	DEFAULT_OBJDUMP=arm-none-eabi-objdump
fi

OBJDUMP=${OBJDUMP:-$DEFAULT_OBJDUMP}

elf_file=$1

function get_symbol {
	name=$1
	size=$("$OBJDUMP" -t "$elf_file" | awk -v n="$name" '$NF == n {print toupper($1); exit}')
	if [[ -z "$size" ]]; then
		echo "Missing symbol: $name" >&2
		exit 1
	fi
	printf "%d\n" "$((16#$size))"
}

function get_section_length {
	name=$1
	start=$(get_symbol "__${name}_start__")
	end=$(get_symbol "__${name}_end__")
	echo $(( end - start ))
}

function print_usage {
	symbol=$1
	length_symbol=$2
	usage=$(get_section_length $symbol)
	usagemb=$(printf "%.3f" "$(( (usage * 1000000) / 1024 / 1024 ))e-6")
	length=$(get_symbol $length_symbol)
	lengthmb=$(printf "%.3f" "$(( (length * 1000000) / 1024 / 1024 ))e-6")
	free=$(( length - usage ))
	freemb=$(printf "%.3f" "$(( (free * 1000000) / 1024 / 1024 ))e-6")
	echo -e ""
	echo -e "External flash usage"
	printf  "    Capacity: %12d Bytes (%7.3f MB)\n" $length $lengthmb
	printf  "    Usage:    %12d Bytes (%7.3f MB)\n" $usage $usagemb
	printf  "    Free:     %12d Bytes (%7.3f MB)\n" $free $freemb
	echo -e ""
}

print_usage extflash __EXTFLASH_LENGTH__
"#;
    fs::write(&extflash_size_script, extflash_size_script_patched)
        .map_err(|error| format!("failed to write {}: {error}", extflash_size_script.display()))?;
    Ok(())
}

fn locate_make_exe() -> Option<PathBuf> {
    let candidates = [
        tool_candidate(&["make", "bin", "mingw32-make.exe"]),
        tool_candidate(&["make", "bin", "make.exe"]),
        tool_candidate(&["mingw64", "bin", "mingw32-make.exe"]),
        tool_candidate(&["mingw64", "bin", "make.exe"]),
        tool_candidate(&["stm32", "make", "bin", "make.exe"]),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_file_under(&runtime_tools_dir(), "mingw32-make.exe"))
        .or_else(|| find_file_under(&runtime_tools_dir(), "make.exe"))
}

fn locate_gcc_bin_dir() -> Option<PathBuf> {
    let candidates = [
        tool_candidate(&["gcc-arm-none-eabi", "bin"]),
        tool_candidate(&["arm-none-eabi-gcc", "bin"]),
        tool_candidate(&["gcc", "bin"]),
        tool_candidate(&["stm32", "gcc", "bin"]),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("arm-none-eabi-gcc.exe").exists())
        .or_else(|| {
            find_file_under(&runtime_tools_dir(), "arm-none-eabi-gcc.exe")
                .and_then(|path| path.parent().map(Path::to_path_buf))
        })
}

fn locate_git_bin_dir() -> Option<PathBuf> {
    let candidates = [
        tool_candidate(&["git", "bin"]),
        tool_candidate(&["git", "cmd"]),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("bash.exe").exists() && path.join("sh.exe").exists())
        .or_else(|| find_dir_with_files_under(&runtime_tools_dir(), &["bash.exe", "sh.exe"]))
}

fn emit_build_progress(app: &tauri::AppHandle, progress: f64, message: impl Into<String>) {
    let _ = app.emit(
        "build-progress",
        BuildProgressEvent {
            progress,
            message: message.into(),
        },
    );
}

fn uses_flat_msys(git_bin: &Path) -> bool {
    git_bin.join("msys-2.0.dll").exists()
        && git_bin
            .parent()
            .map(|git_root| !git_root.join("usr").join("bin").join("bash.exe").exists())
            .unwrap_or(false)
}

fn to_bash_path_for(path: &Path, git_bin: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.len() >= 3 && normalized.as_bytes()[1] == b':' && normalized.as_bytes()[2] == b'/' {
        let drive = normalized[..1].to_ascii_lowercase();
        if uses_flat_msys(git_bin) {
            format!("/cygdrive/{drive}{}", &normalized[2..])
        } else {
            format!("/{drive}{}", &normalized[2..])
        }
    } else {
        normalized
    }
}

fn ensure_msys_tmp_dir(git_bin: &Path) -> Result<(), String> {
    if !uses_flat_msys(git_bin) {
        return Ok(());
    }
    let Some(git_root) = git_bin.parent() else {
        return Ok(());
    };
    let Some(tools_root) = git_root.parent() else {
        return Ok(());
    };
    fs::create_dir_all(tools_root.join("tmp"))
        .map_err(|error| format!("failed to create MSYS tmp dir: {error}"))
}

fn read_lossy_output_line<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    let read = reader.read_until(b'\n', &mut bytes)?;
    if read == 0 {
        return Ok(None);
    }
    if bytes.ends_with(b"\n") {
        bytes.pop();
        if bytes.ends_with(b"\r") {
            bytes.pop();
        }
    }
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

fn build_job_count() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .clamp(1, 24)
}

fn run_retro_go_build(
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

fn emulator_icon_path(emulator: &str) -> PathBuf {
    let file_name = match emulator {
        _ => format!("{}.png", sanitize_file_part(emulator)),
    };
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.parent().map(|path| path.join("public").join("emulators").join(&file_name)),
        std::env::current_dir()
            .ok()
            .map(|path| path.join("public").join("emulators").join(&file_name)),
        Some(host_root().join("public").join("emulators").join(&file_name)),
        Some(host_root().join("gw-studio-tauri").join("public").join("emulators").join(&file_name)),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|path| path.exists())
        .unwrap_or_else(|| manifest_dir.join("..").join("public").join("emulators").join(file_name))
}

fn write_coverflow_source_image(source_image: &Path, destination: &Path) -> Result<(), String> {
    let bytes = fs::read(source_image)
        .map_err(|error| format!("failed to read preview image {}: {error}", source_image.display()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("failed to decode preview image {}: {error}", source_image.display()))?;
    let resized = image.resize(96, 96, FilterType::Lanczos3);
    let mut cursor = Cursor::new(Vec::new());
    resized
        .write_to(&mut cursor, ImageFormat::Jpeg)
        .map_err(|error| format!("failed to encode coverflow jpg: {error}"))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create cover image dir {}: {error}", parent.display()))?;
    }
    fs::write(destination, cursor.into_inner())
        .map_err(|error| format!("failed to write cover image {}: {error}", destination.display()))?;
    Ok(())
}

fn write_imported_rom(emulator: &str, source_label: &str, file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let dest_dir = rom_imports_dir()
        .join(sanitize_file_part(emulator))
        .join(sanitize_file_part(source_label));
    fs::create_dir_all(&dest_dir).map_err(|error| format!("failed to create ROM import dir: {error}"))?;
    let dest_path = dest_dir.join(sanitize_file_part(file_name));
    fs::write(&dest_path, bytes).map_err(|error| format!("failed to write imported ROM: {error}"))?;
    Ok(dest_path)
}

#[tauri::command]
fn compute_build_metrics(request: BuildMetricsRequest) -> Result<BuildMetrics, String> {
    let mut rom_bytes = 0_u64;
    let mut image_bytes = 0_u64;
    let mut image_count = 0_usize;
    let rom_count = request.rom_paths.len();

    for rom_path in &request.rom_paths {
        match fs::metadata(&rom_path) {
            Ok(metadata) if metadata.is_file() => {
                rom_bytes = rom_bytes.saturating_add(metadata.len());
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    for title in request.titles {
        let path = thumbnail_cache_path(&request.emulator, &title);
        match fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => {
                image_bytes = image_bytes.saturating_add(metadata.len());
                image_count += 1;
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    Ok(BuildMetrics {
        rom_bytes,
        image_bytes,
        rom_count,
        image_count,
    })
}

#[tauri::command]
async fn build_firmware_bundle(app: tauri::AppHandle, request: BuildBundleRequest) -> Result<BuildBundleResult, String> {
    tauri::async_runtime::spawn_blocking(move || build_firmware_bundle_blocking(&app, request))
        .await
        .map_err(|error| format!("failed to join build firmware task: {error}"))?
}

fn build_firmware_bundle_blocking(app: &tauri::AppHandle, request: BuildBundleRequest) -> Result<BuildBundleResult, String> {
    if request.entries.is_empty() {
        return Err("no ROM entries provided for bundle".to_string());
    }

    emit_build_progress(app, 2.0, "Preparing build workspace");

    let coverflow_enabled = request.coverflow_enabled.unwrap_or(false);
    let firmware_profile = firmware_profile_code(&request.firmware_profile).to_string();
    let bundle_name = format!("bundle_{}_{}", firmware_profile, bundle_stamp());
    let bundle_dir = bundles_dir().join(&bundle_name);
    let retro_go_workspace_dir = build_workspaces_dir().join(format!("retro_go_{}", bundle_name));
    let roms_root = bundle_dir.join("content").join("roms");
    let previews_root = bundle_dir.join("content").join("previews");
    fs::create_dir_all(&roms_root).map_err(|error| format!("failed to create bundle rom dir: {error}"))?;
    fs::create_dir_all(&previews_root).map_err(|error| format!("failed to create bundle preview dir: {error}"))?;
    ensure_clean_dir(&retro_go_workspace_dir)?;
    let repo_path = locate_retro_go_repo().ok_or_else(|| "Retro-Go fork repository not found".to_string())?;
    copy_dir_filtered(&repo_path, &retro_go_workspace_dir)?;
    patch_retro_go_workspace_for_windows(&retro_go_workspace_dir, coverflow_enabled)?;
    fs::create_dir_all(retro_go_workspace_dir.join("roms"))
        .map_err(|error| format!("failed to create Retro-Go fork rom dir: {error}"))?;
    let mut rom_bytes = 0_u64;
    let mut image_bytes = 0_u64;
    let mut image_count = 0_usize;
    let mut coverflow_count = 0_usize;
    let mut copied_roms = Vec::new();
    let mut copied_previews = Vec::new();
    let mut copied_coverflow = Vec::new();
    let mut grouped_counts = BTreeMap::<String, usize>::new();
    let total_entries = request.entries.len().max(1) as f64;
    let has_msx_entries = request.entries.iter().any(|entry| entry.emulator == "msx");
    if has_msx_entries {
        copy_msx_bios_to_workspace(&retro_go_workspace_dir)?;
    }
    let has_coleco_entries = request.entries.iter().any(|entry| entry.emulator == "col");
    if has_coleco_entries {
        copy_coleco_bios_to_workspace(&retro_go_workspace_dir)?;
    }

    for (index, entry) in request.entries.iter().enumerate() {
        let stage_progress = 8.0 + ((index as f64 / total_entries) * 52.0);
        emit_build_progress(
            app,
            stage_progress,
            format!("Packing {} ROM(s): {}", index + 1, entry.title),
        );
        let source_rom = PathBuf::from(&entry.rom_path);
        let rom_name = source_rom
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("rom.bin")
            .to_string();
        let emulator_dir = roms_root.join(sanitize_file_part(&entry.emulator));
        fs::create_dir_all(&emulator_dir).map_err(|error| format!("failed to create emulator rom dir: {error}"))?;
        let dest_rom = emulator_dir.join(sanitize_file_part(&rom_name));
        fs::copy(&source_rom, &dest_rom).map_err(|error| format!("failed to copy ROM into bundle: {error}"))?;
        let rom_size = fs::metadata(&dest_rom)
            .map_err(|error| format!("failed to stat copied ROM: {error}"))?
            .len();
        rom_bytes = rom_bytes.saturating_add(rom_size);
        copied_roms.push(dest_rom.to_string_lossy().to_string());
        *grouped_counts.entry(entry.emulator.clone()).or_insert(0) += 1;

        if let Some(retro_go_dirname) = retro_go_emulator_dir(&entry.emulator) {
            let workspace_rom_dir = retro_go_workspace_dir.join("roms").join(retro_go_dirname);
            fs::create_dir_all(&workspace_rom_dir)
                .map_err(|error| format!("failed to create Retro-Go fork emulator dir: {error}"))?;
            let workspace_rom = workspace_rom_dir.join(retro_go_workspace_rom_name(&entry.emulator, &rom_name));
            fs::copy(&source_rom, &workspace_rom)
                .map_err(|error| format!("failed to copy ROM into Retro-Go fork workspace: {error}"))?;

            if coverflow_enabled {
                let preview_source = thumbnail_cache_path(&entry.emulator, &entry.title);
                let cover_source = if preview_source.exists() {
                    Some(preview_source)
                } else {
                    let icon_path = emulator_icon_path(&entry.emulator);
                    icon_path.exists().then_some(icon_path)
                };
                if let Some(cover_source) = cover_source {
                    let cover_dest = workspace_rom.with_extension("jpg");
                    write_coverflow_source_image(&cover_source, &cover_dest)?;
                    coverflow_count += 1;
                    copied_coverflow.push(cover_dest.to_string_lossy().to_string());
                }
            }
        }

        let preview_source = thumbnail_cache_path(&entry.emulator, &entry.title);
        let preview_dest = previews_root
            .join(sanitize_file_part(&entry.emulator))
            .join(format!("{}.png", sanitize_thumbnail_part(&entry.title)));
        if copy_if_exists(&preview_source, &preview_dest)? {
            let preview_size = fs::metadata(&preview_dest)
                .map_err(|error| format!("failed to stat copied preview: {error}"))?
                .len();
            image_bytes = image_bytes.saturating_add(preview_size);
            image_count += 1;
            copied_previews.push(preview_dest.to_string_lossy().to_string());
        }
    }

    let total_bytes = rom_bytes.saturating_add(image_bytes);
    let installed_spi_bytes = (request.installed_spi_mb.max(0.0) * 1024.0 * 1024.0).round() as u64;
    let firmware_reserved_bytes = (request.firmware_reserved_mb.max(0.0) * 1024.0 * 1024.0).round() as u64;
    let used_bytes = firmware_reserved_bytes.saturating_add(total_bytes);
    let overflow_bytes = used_bytes.saturating_sub(installed_spi_bytes);

    let summary_path = bundle_dir.join("summary.txt");
    let manifest_path = bundle_dir.join("bundle_manifest.json");
    let flash_script_path = bundle_dir.join("flash_bundle.ps1");
    let stock_bank1_candidate = explicit_stock_file(request.stock_bank1_path.as_deref())?;
    let stock_spi_candidate = explicit_stock_file(request.stock_spi_path.as_deref())?;
    let extflash_build_path = retro_go_workspace_dir
        .join("build")
        .join("gw_retro_go_extflash.bin");
    let intflash_build_path = retro_go_workspace_dir
        .join("build")
        .join("gw_retro_go_intflash.bin");
    let build_log_path = retro_go_workspace_dir.join("build_gw_studio.log");
    let patch_workspace_dir = build_workspaces_dir().join(format!("game_watch_patch_{}", bundle_name));
    let extflash_offset_bytes = firmware_reserved_bytes;
    let extflash_size_mb = (request.installed_spi_mb - request.firmware_reserved_mb)
        .max(1.0)
        .round() as u32;
    let patched_bank1_candidate = run_game_watch_patch_build(
        app,
        &patch_workspace_dir,
        &stock_bank1_candidate,
        &stock_spi_candidate,
        &firmware_profile,
    )?;
    emit_build_progress(app, 64.0, "Workspace ready, starting Retro-Go fork build");
    let _actual_build_log_path = run_retro_go_build(
        app,
        &retro_go_workspace_dir,
        extflash_size_mb,
        extflash_offset_bytes,
        2,
        &firmware_profile,
        coverflow_enabled,
    )?;
    let extflash_built = extflash_build_path.exists();
    let extflash_build_size_bytes = if extflash_built {
        fs::metadata(&extflash_build_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    } else {
        0
    };
    let firmware_outputs_dir = bundle_dir.join("firmware");
    fs::create_dir_all(&firmware_outputs_dir)
        .map_err(|error| format!("failed to create firmware outputs dir: {error}"))?;
    let bank1_output_path = firmware_outputs_dir.join("bank1d.bin");
    let bank2_output_path = firmware_outputs_dir.join(firmware_output_name("bank2", &firmware_profile));
    let spi_output_path = firmware_outputs_dir.join(firmware_output_name("spi", &firmware_profile));
    let bank1_output = if copy_if_exists(&patched_bank1_candidate, &bank1_output_path)? {
        Some(bank1_output_path.clone())
    } else {
        None
    };
    let bank2_output = if copy_if_exists(&intflash_build_path, &bank2_output_path)? {
        Some(bank2_output_path.clone())
    } else {
        None
    };
    let patched_spi_candidate = patch_workspace_dir.join("build").join("external_flash_patched.bin");
    let patched_spi_size = fs::metadata(&patched_spi_candidate).map(|metadata| metadata.len()).unwrap_or(0);
    let spi_prefix_source = if patched_spi_size >= extflash_offset_bytes {
        patched_spi_candidate.clone()
    } else {
        stock_spi_candidate.clone()
    };
    compose_spi_image(&spi_prefix_source, &extflash_build_path, &spi_output_path, extflash_offset_bytes)?;
    let spi_output = spi_output_path.clone();
    let spi_output_size_bytes = fs::metadata(&spi_output).map(|metadata| metadata.len()).unwrap_or(0);

    let mut summary_lines = vec![
        format!("Bundle: {bundle_name}"),
        format!("Firmware profile: {}", request.firmware_profile),
        format!("Firmware code: {}", firmware_profile.to_ascii_uppercase()),
        format!("Installed SPI MB: {}", request.installed_spi_mb),
        format!("Firmware reserved MB: {}", request.firmware_reserved_mb),
        format!("Retro-Go fork EXTFLASH_SIZE_MB: {}", extflash_size_mb),
        format!("Retro-Go fork EXTFLASH_OFFSET: {}", extflash_offset_bytes),
        "Retro-Go fork INTFLASH_BANK: 2".to_string(),
        format!("ROM count: {}", request.entries.len()),
        format!("Image count: {}", image_count),
        format!("Coverflow enabled: {}", coverflow_enabled),
        format!("Coverflow image count: {}", coverflow_count),
        format!("ROM bytes: {}", rom_bytes),
        format!("Image bytes: {}", image_bytes),
        format!("Payload bytes: {}", total_bytes),
        format!("Used bytes with firmware reserve: {}", used_bytes),
        format!("Overflow bytes: {}", overflow_bytes),
        format!(
            "Retro-Go fork repo: {}",
            repo_path
                .to_string_lossy()
                .to_string()
        ),
        format!(
            "Stock Bank1 source: {}",
            stock_bank1_candidate
                .to_string_lossy()
                .to_string()
        ),
        format!(
            "Stock SPI source: {}",
            stock_spi_candidate
                .to_string_lossy()
                .to_string()
        ),
        format!(
            "Dualboot Bank1 candidate: {}",
            bank1_output
                .as_ref()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| "missing".to_string())
        ),
        format!(
            "Bank2 candidate: {}",
            bank2_output
                .as_ref()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| "missing".to_string())
        ),
        format!("Retro-Go fork workspace: {}", retro_go_workspace_dir.display()),
        format!("Build log: {}", build_log_path.display()),
        format!("Expected intflash output: {}", intflash_build_path.display()),
        format!("Expected extflash output: {}", extflash_build_path.display()),
        format!("Extflash built: {}", extflash_built),
        format!("SPI prefix source: {}", spi_prefix_source.display()),
        format!("Retro-Go fork extflash payload size: {}", extflash_build_size_bytes),
        format!("Flashable SPI image size: {}", spi_output_size_bytes),
        "Flashable SPI image includes stock SPI prefix".to_string(),
        String::new(),
        "Emulators:".to_string(),
    ];
    for (emulator, count) in grouped_counts {
        summary_lines.push(format!("- {emulator}: {count} rom(s)"));
    }
    fs::write(&summary_path, summary_lines.join("\n")).map_err(|error| format!("failed to write bundle summary: {error}"))?;

    let manifest = serde_json::json!({
        "bundle_name": bundle_name,
        "firmware_profile": request.firmware_profile,
        "firmware_code": firmware_profile,
        "installed_spi_mb": request.installed_spi_mb,
        "firmware_reserved_mb": request.firmware_reserved_mb,
        "retro_go_extflash_size_mb": extflash_size_mb,
        "retro_go_extflash_offset_bytes": extflash_offset_bytes,
        "retro_go_intflash_bank": 2,
        "rom_count": request.entries.len(),
        "image_count": image_count,
        "coverflow_enabled": coverflow_enabled,
        "coverflow_count": coverflow_count,
        "rom_bytes": rom_bytes,
        "image_bytes": image_bytes,
        "payload_bytes": total_bytes,
        "used_bytes": used_bytes,
        "overflow_bytes": overflow_bytes,
        "bundle_dir": bundle_dir,
        "retro_go_workspace_dir": retro_go_workspace_dir,
        "build_log_path": build_log_path,
        "bank1_patch_workspace_dir": patch_workspace_dir,
        "retro_go_repo": repo_path,
        "bank1_candidate_path": bank1_output,
        "stock_bank1_source_path": stock_bank1_candidate,
        "stock_spi_source_path": stock_spi_candidate,
        "spi_prefix_source_path": spi_prefix_source,
        "bank1_dualboot": true,
        "bank1_dualboot_entry": "LEFT+GAME",
        "bank2_candidate_path": bank2_output,
        "extflash_build_path": spi_output,
        "extflash_built": extflash_built,
        "extflash_build_size_bytes": spi_output_size_bytes,
        "retro_go_extflash_payload_size_bytes": extflash_build_size_bytes,
        "spi_full_image": true,
        "files": {
            "summary": summary_path,
            "flash_script": flash_script_path,
            "build_log": build_log_path,
            "roms": copied_roms,
            "previews": copied_previews,
            "coverflow_images": copied_coverflow,
        },
    });
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).map_err(|error| format!("failed to encode bundle manifest: {error}"))?,
    )
    .map_err(|error| format!("failed to write bundle manifest: {error}"))?;

    fs::write(
        &flash_script_path,
        [
            "$ErrorActionPreference = 'Stop'",
            "# Build bundle prepared by GW Studio",
            &format!("# Retro-Go fork workspace: {}", retro_go_workspace_dir.display()),
            &format!("# Expected extflash build output: {}", spi_output.display()),
            &format!("# Build log: {}", build_log_path.display()),
            "# Flash flow is not fully wired yet; use extflash output and build log as current output.",
            "",
        ]
        .join("\n"),
    )
    .map_err(|error| format!("failed to write flash script placeholder: {error}"))?;

    Ok(BuildBundleResult {
        bundle_dir: bundle_dir.to_string_lossy().to_string(),
        summary_path: summary_path.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        retro_go_workspace_dir: retro_go_workspace_dir.to_string_lossy().to_string(),
        build_log_path: build_log_path.to_string_lossy().to_string(),
        extflash_built,
        bank1_candidate_path: bank1_output
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        bank2_candidate_path: bank2_output
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        extflash_build_path: spi_output.to_string_lossy().to_string(),
        extflash_build_size_bytes: spi_output_size_bytes,
        rom_count: request.entries.len(),
        image_count,
        coverflow_count,
        romart_count: coverflow_count,
        rom_bytes,
        image_bytes,
        total_bytes,
    })
}

#[tauri::command]
fn latest_firmware_bundle() -> Result<FirmwareBundleLookupResult, String> {
    let root = bundles_dir();
    if !root.exists() {
        return Ok(FirmwareBundleLookupResult {
            found: false,
            message: "No firmware bundle directory exists. Run Build Firmware first.".to_string(),
            bundle_dir: String::new(),
            manifest_path: String::new(),
            bank1_candidate_path: String::new(),
            bank2_candidate_path: String::new(),
            extflash_build_path: String::new(),
            extflash_build_size_bytes: 0,
        });
    }

    let mut manifests = Vec::new();
    for entry in fs::read_dir(&root).map_err(|error| format!("failed to read bundles dir: {error}"))? {
        let entry = entry.map_err(|error| format!("failed to read bundle dir entry: {error}"))?;
        let path = entry.path().join("bundle_manifest.json");
        if path.exists() {
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            manifests.push((modified, path));
        }
    }
    manifests.sort_by(|a, b| b.0.cmp(&a.0));
    let mut skipped_reason = String::new();

    for (_, manifest_path) in manifests {
        let manifest_text = match fs::read_to_string(&manifest_path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let manifest: serde_json::Value = match serde_json::from_str(&manifest_text) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let path_field = |name: &str| -> String {
            manifest
                .get(name)
                .and_then(|value| value.as_str())
                .filter(|value| Path::new(value).is_file())
                .unwrap_or("")
                .to_string()
        };
        let bank1 = path_field("bank1_candidate_path");
        let bank2 = path_field("bank2_candidate_path");
        let spi = path_field("extflash_build_path");
        let spi_full_image = manifest
            .get("spi_full_image")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let has_spi_prefix_source = manifest
            .get("spi_prefix_source_path")
            .and_then(|value| value.as_str())
            .filter(|value| Path::new(value).is_file())
            .is_some();
        let firmware_code = manifest
            .get("firmware_code")
            .and_then(|value| value.as_str())
            .map(firmware_profile_code)
            .unwrap_or("m");
        let bank1_is_dualboot = manifest
            .get("bank1_dualboot")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            || Path::new(&bank1)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("bank1d.bin"))
                .unwrap_or(false);
        if bank1.is_empty() || bank2.is_empty() || spi.is_empty() || !bank1_is_dualboot {
            if skipped_reason.is_empty() {
                skipped_reason = format!(
                    "Latest bundle is not flashable for dualboot: Bank1={}, Bank2={}, SPI={}, dualboot={}. Rebuild must produce bank1d.bin.",
                    if bank1.is_empty() { "missing" } else { "present" },
                    if bank2.is_empty() { "missing" } else { "present" },
                    if spi.is_empty() { "missing" } else { "present" },
                    bank1_is_dualboot,
                );
            }
            continue;
        }
        if !spi_full_image || !has_spi_prefix_source {
            if skipped_reason.is_empty() {
                skipped_reason = "Latest bundle uses old SPI image format. Rebuild firmware to create a patched full flashable SPI image.".to_string();
            }
            continue;
        }
        let bank1_size = fs::metadata(&bank1).map(|metadata| metadata.len()).unwrap_or(0);
        let expected_bank1_size = if firmware_code == "z" { 128 * 1024 } else { 256 * 1024 };
        if bank1_size != expected_bank1_size {
            if skipped_reason.is_empty() {
                skipped_reason = format!(
                    "Latest bundle has invalid Bank1 size for {}: {} bytes, expected {} bytes.",
                    firmware_code.to_ascii_uppercase(),
                    bank1_size,
                    expected_bank1_size,
                );
            }
            continue;
        }
        let extflash_build_size_bytes = if spi.is_empty() {
            0
        } else {
            fs::metadata(&spi).map(|metadata| metadata.len()).unwrap_or(0)
        };
        let bundle_dir = manifest
            .get("bundle_dir")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        return Ok(FirmwareBundleLookupResult {
            found: true,
            message: "Dualboot firmware bundle found.".to_string(),
            bundle_dir,
            manifest_path: manifest_path.to_string_lossy().to_string(),
            bank1_candidate_path: bank1,
            bank2_candidate_path: bank2,
            extflash_build_path: spi,
            extflash_build_size_bytes,
        });
    }

    let message = if skipped_reason.is_empty() {
        latest_bank1_patch_failure_message()
            .unwrap_or_else(|| "No dualboot firmware bundle found. Run Build Firmware first.".to_string())
    } else if let Some(patch_failure) = latest_bank1_patch_failure_message() {
        format!("{skipped_reason}\nLatest Bank1 patch error:\n{patch_failure}")
    } else {
        skipped_reason
    };

    Ok(FirmwareBundleLookupResult {
        found: false,
        message,
        bundle_dir: String::new(),
        manifest_path: String::new(),
        bank1_candidate_path: String::new(),
        bank2_candidate_path: String::new(),
        extflash_build_path: String::new(),
        extflash_build_size_bytes: 0,
    })
}

fn latest_bank1_patch_failure_message() -> Option<String> {
    let root = build_workspaces_dir();
    if !root.exists() {
        return None;
    }
    let mut logs = Vec::new();
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path().join("build_gw_studio_patch.log");
        if path.exists() {
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            logs.push((modified, path));
        }
    }
    logs.sort_by(|a, b| b.0.cmp(&a.0));
    let path = logs.first()?.1.clone();
    let text = fs::read_to_string(&path).ok()?;
    let tail = text
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("{}\n{}", path.display(), tail))
}

fn normalize_device_uid(value: &str) -> Option<String> {
    let hex = value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_uppercase();
    if hex.len() >= 24 {
        Some(hex[..24].to_string())
    } else {
        None
    }
}

fn device_uid_from_details(details: &BTreeMap<String, String>, raw_text: &str) -> Option<String> {
    for key in [
        "MCU UID",
        "Device UID",
        "Device Unique ID",
        "Unique ID",
        "Unique device ID",
        "UID",
    ] {
        if let Some(value) = details.get(key).and_then(|value| normalize_device_uid(value)) {
            return Some(value);
        }
    }

    for line in raw_text.lines() {
        let normalized = line.to_ascii_lowercase();
        if !(normalized.contains("uid") || normalized.contains("unique")) {
            continue;
        }
        if let Some(value) = normalize_device_uid(line) {
            return Some(value);
        }
    }

    None
}

fn gnwmanager_argv() -> Vec<String> {
    let python = locate_python_exe();
    if python.exists() {
        return vec![
            python.to_string_lossy().to_string(),
            "-u".to_string(),
            "-m".to_string(),
            "gnwmanager".to_string(),
        ];
    }
    vec![
        "py".to_string(),
        "-3".to_string(),
        "-u".to_string(),
        "-m".to_string(),
        "gnwmanager".to_string(),
    ]
}

fn parse_details(output: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in output.lines() {
        if let Some((left, right)) = line.split_once(':') {
            let key = left.trim().to_string();
            let value = right.trim().to_string();
            if !key.is_empty() && !value.is_empty() {
                map.insert(key, value);
            }
        }
    }
    map
}

fn detail<'a>(details: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = details.get(*key) {
            return Some(value.clone());
        }
    }
    None
}

fn run_gnwmanager_info(backend: &str, frequency: u32) -> Result<Output, String> {
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
        .arg("info")
        ;

    hide_command_window(&mut command)
        .output()
        .map_err(|error| format!("failed to run gnwmanager info: {error}"))
}

fn run_gnwmanager_info_with_retries(backend: &str, frequency: u32, attempts: usize) -> Result<Output, String> {
    let mut last_text = String::new();
    let total = attempts.max(1);
    for attempt in 0..total {
        let output = run_gnwmanager_info(backend, frequency)?;
        if output.status.success() {
            return Ok(output);
        }
        last_text = output_text(&output);
        if attempt + 1 < total {
            thread::sleep(Duration::from_millis(350));
        }
    }
    Err(format!(
        "gnwmanager info failed after {total} attempts: {}",
        last_text.lines().last().unwrap_or("unknown error")
    ))
}

fn read_info_frequency_attempts(requested_frequency: u32) -> Vec<u32> {
    let mut attempts = Vec::new();
    for value in [
        requested_frequency,
        8_000_000,
        4_000_000,
        2_000_000,
        1_000_000,
        500_000,
        240_000,
        100_000,
    ] {
        if value > 0 && !attempts.contains(&value) {
            attempts.push(value);
        }
    }
    attempts
}

fn run_gnwmanager_info_with_frequency_fallback(
    backend: &str,
    frequency: u32,
) -> Result<(Output, u32), String> {
    let mut last_error = String::new();
    for candidate_frequency in read_info_frequency_attempts(frequency) {
        match run_gnwmanager_info_with_retries(backend, candidate_frequency, 2) {
            Ok(output) => return Ok((output, candidate_frequency)),
            Err(error) => {
                last_error = format!("freq {candidate_frequency}: {error}");
            }
        }
    }
    Err(format!("gnwmanager info failed after frequency fallback: {last_error}"))
}

fn run_direct_info_under_reset(frequency: u32) -> Result<String, String> {
    let script = r#"
from gnwmanager.gnw import GnW
from gnwmanager.ocdbackend.pyocd_backend import PyOCDBackend

try:
    from gnwmanager.cli.devices import DeviceModel
except Exception:
    DeviceModel = None

backend = PyOCDBackend(connect_mode="under-reset")
gnw = GnW(backend)
try:
    backend.open()
    backend.set_frequency(__FREQUENCY__)
    print("OCD Backend Version:         pyocd-direct")
    print("Debug Probe:                 " + str(getattr(backend, "probe_name", "UNKNOWN")))
    try:
        probe = getattr(backend, "probe", None)
        link = getattr(probe, "_link", None) if probe is not None else None
        voltage = getattr(link, "target_voltage", None) if link is not None else None
        print("Target voltage:             " + (f"{float(voltage):.2f} V" if voltage is not None else "UNKNOWN"))
    except Exception:
        print("Target voltage:             UNKNOWN")
    try:
        data = backend.read_memory(0x08FFF800, 12)
        uid = "".join(f"{int.from_bytes(data[index:index + 4], 'little'):08X}" for index in range(0, 12, 4))
        print("Device UID:                 " + uid)
    except Exception:
        print("Device UID:                 UNKNOWN")
    try:
        cpuid = int.from_bytes(backend.read_memory(0xE000ED00, 4), "little")
        print("CPU ID:                     CPUID 0x%08X" % cpuid)
    except Exception:
        print("CPU ID:                     UNKNOWN")
    try:
        gnw.start_gnwmanager()
        if DeviceModel is not None:
            try:
                print("Detected Stock Firmware:    " + str(DeviceModel.autodetect(gnw)).upper())
            except Exception:
                print("Detected Stock Firmware:    UNKNOWN")
        else:
            print("Detected Stock Firmware:    UNKNOWN")
        try:
            print("External Flash Size (MB):   " + str(gnw.external_flash_size / (1 << 20)))
        except Exception:
            print("External Flash Size (MB):   UNKNOWN")
        try:
            print("Locked?:                    " + ("LOCKED" if gnw.is_locked() else "UNLOCKED"))
        except Exception:
            print("Locked?:                    UNKNOWN")
        print("Filesystem Size (B):        UNKNOWN")
    except Exception as exc:
        print("Detected Stock Firmware:    UNKNOWN")
        print("External Flash Size (MB):   UNKNOWN")
        print("Locked?:                    UNKNOWN")
        print("Filesystem Size (B):        UNKNOWN")
        print("Direct info warning:        " + type(exc).__name__ + ": " + str(exc))
finally:
    try:
        backend.close()
    except Exception:
        pass
"#
    .replace("__FREQUENCY__", &frequency.to_string());

    let python = locate_python_exe();
    let mut command = Command::new(&python);
    command.arg("-u").arg("-c").arg(script);
    let output = hide_command_window(&mut command)
        .output()
        .map_err(|error| format!("failed to run direct under-reset info: {error}"))?;
    let text = output_text(&output);
    if !output.status.success() {
        return Err(format!("direct under-reset info failed: {text}"));
    }
    Ok(text)
}

fn run_direct_info_with_frequency_fallback(frequency: u32) -> Result<(String, u32), String> {
    let mut last_error = String::new();
    for candidate_frequency in read_info_frequency_attempts(frequency) {
        match run_direct_info_under_reset(candidate_frequency) {
            Ok(text) => return Ok((text, candidate_frequency)),
            Err(error) => last_error = format!("freq {candidate_frequency}: {error}"),
        }
    }
    Err(format!("direct under-reset info failed after frequency fallback: {last_error}"))
}

fn read_device_uid_under_reset(frequency: u32) -> Result<String, String> {
    let script = format!(
        r#"
from gnwmanager.ocdbackend.pyocd_backend import PyOCDBackend

backend = PyOCDBackend(connect_mode="under-reset")
try:
    backend.open()
    backend.set_frequency({frequency})
    data = backend.read_memory(0x08FFF800, 12)
    uid = "".join(f"{{int.from_bytes(data[index:index + 4], 'little'):08X}}" for index in range(0, 12, 4))
    print("device_uid=" + uid)
finally:
    try:
        backend.close()
    except Exception:
        pass
"#
    );

    let python = locate_python_exe();
    let mut command = Command::new(&python);
    command.arg("-c").arg(script);
    let output = hide_command_window(&mut command)
        .output()
        .map_err(|error| format!("failed to run pyocd UID read: {error}"))?;
    let text = output_text(&output);
    if !output.status.success() {
        return Err(format!("pyocd UID read failed: {text}"));
    }

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "device_uid" {
            if let Some(uid) = normalize_device_uid(value) {
                return Ok(uid);
            }
        }
    }

    Err(format!("pyocd UID read returned no UID: {text}"))
}

fn read_target_voltage_pyocd(frequency: u32) -> Result<String, String> {
    let script = format!(
        r#"
from gnwmanager.ocdbackend.pyocd_backend import PyOCDBackend

backend = PyOCDBackend(connect_mode="under-reset")
try:
    backend.open()
    backend.set_frequency({frequency})
    probe = backend.probe
    link = getattr(probe, "_link", None)
    voltage = getattr(link, "target_voltage", None) if link is not None else None
    if voltage is None:
        raise RuntimeError("target voltage is not exposed by this probe")
    print(f"target_voltage={{float(voltage):.2f}} V")
finally:
    try:
        backend.close()
    except Exception:
        pass
"#
    );

    let python = locate_python_exe();
    let mut command = Command::new(&python);
    command.arg("-c").arg(script);
    let output = hide_command_window(&mut command)
        .output()
        .map_err(|error| format!("failed to run pyocd voltage read: {error}"))?;
    let text = output_text(&output);
    if !output.status.success() {
        return Err(format!("pyocd voltage read failed: {text}"));
    }

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "target_voltage" {
            let value = value.trim();
            if !value.is_empty() {
                return Ok(value.to_string());
            }
        }
    }

    Err(format!("pyocd voltage read returned no voltage: {text}"))
}

fn run_pyocd_internal_dump_under_reset(
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

fn run_pyocd_internal_flash_under_reset(
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

fn output_text(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    text
}

fn build_frequency_attempts(preferred: u32) -> Vec<u32> {
    let mut values = Vec::new();
    for value in [
        preferred,
        8000_000,
        240_000,
        180_000,
        120_000,
        100_000,
        80_000,
        40_000,
        20_000,
        10_000,
    ] {
        if value > 0 && !values.contains(&value) {
            values.push(value);
        }
    }
    values
}

fn build_backend_frequency_attempts(backend: &str, preferred: u32) -> Vec<u32> {
    if backend.eq_ignore_ascii_case("pyocd") {
        let mut values = Vec::new();
        for value in [
            100_000,
            80_000,
            120_000,
            180_000,
            240_000,
            500_000,
            1_000_000,
            preferred,
        ] {
            if value > 0 && !values.contains(&value) {
                values.push(value);
            }
        }
        return values;
    }
    build_frequency_attempts(preferred)
}

fn emit_backup_progress(
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

fn emit_backup_debug(app: &tauri::AppHandle, phase: &str, source: &str, line: &str) {
    let _ = app.emit(
        "backup-debug",
        BackupDebugEvent {
            phase: phase.to_string(),
            line: line.to_string(),
            source: source.to_string(),
        },
    );
}

fn emit_firmware_write_progress(
    app: &tauri::AppHandle,
    phase: &str,
    stage: &str,
    progress: f64,
    speed_bps: f64,
    frequency: u32,
    backend: &str,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "firmware-write-progress",
        FirmwareWriteProgressEvent {
            phase: phase.to_string(),
            stage: stage.to_string(),
            progress,
            speed_bps,
            frequency,
            backend: backend.to_string(),
            message: message.into(),
        },
    );
}

fn emit_backup_progress_throttled(
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

fn classify_dump_progress(phase: &str, line: &str) -> Option<(f64, String)> {
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

fn parse_tqdm_spi_progress(line: &str) -> Option<(f64, f64, String)> {
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

fn forward_stream_updates<R: Read + Send + 'static>(
    reader: R,
    sender: mpsc::Sender<(bool, String)>,
    is_stdout: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = [0_u8; 1024];
        let mut pending = Vec::<u8>::new();

        loop {
            let bytes_read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => size,
                Err(_) => break,
            };

            for byte in &buffer[..bytes_read] {
                if *byte == b'\r' || *byte == b'\n' {
                    if !pending.is_empty() {
                        let line = String::from_utf8_lossy(&pending).trim().to_string();
                        if !line.is_empty() {
                            let _ = sender.send((is_stdout, line));
                        }
                        pending.clear();
                    }
                } else {
                    pending.push(*byte);
                }
            }
        }

        if !pending.is_empty() {
            let line = String::from_utf8_lossy(&pending).trim().to_string();
            if !line.is_empty() {
                let _ = sender.send((is_stdout, line));
            }
        }
    })
}

fn run_dump_with_progress(
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

fn run_flash_command(backend: &str, frequency: u32, target: &str, source: &PathBuf) -> Result<Output, String> {
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

fn run_gnwmanager_spi_erase_chunks(
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

fn run_gnwmanager_spi_read(
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

fn run_gnwmanager_spi_write(
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

fn run_single_flash_phase(
    app: &tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    source_path: String,
    target: &str,
    external_flash_mb: f64,
    external_flash_offset_bytes: u64,
) -> Result<FirmwareWriteResult, String> {
    let is_locked = protection.trim().eq_ignore_ascii_case("LOCKED");
    if is_locked {
        return Err("device is locked; flash write is unavailable while protection is enabled".to_string());
    }

    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Err(format!("firmware file not found: {}", source.display()));
    }
    if !source.is_file() {
        return Err(format!("firmware path is not a file: {}", source.display()));
    }

    if target == "ext" {
        let full_flash_bytes = (external_flash_mb.max(1.0) * 1024.0 * 1024.0).round() as u64;
        run_gnwmanager_spi_erase_chunks(app, frequency, full_flash_bytes)?;
        let output = run_gnwmanager_spi_write(app, frequency, &source, external_flash_offset_bytes)?;
        let text = output_text(&output);
        if output.status.success() {
            return Ok(FirmwareWriteResult {
                summary: format!("SPI flash completed successfully (gnwmanager, freq {frequency})"),
                path: source.to_string_lossy().to_string(),
                target: target.to_string(),
                backend: "gnwmanager".to_string(),
                frequency,
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        return Err(format!(
            "failed to flash SPI with GNWManager: {}",
            text.lines().last().unwrap_or("unknown error")
        ));
    }

    let used_backend = backend;

    if used_backend.eq_ignore_ascii_case("pyocd") && (target == "bank1" || target == "bank2") {
        let direct_bank = if target == "bank1" { 1_u8 } else { 2_u8 };
        let mut direct_frequencies = Vec::new();
        for value in [1_000_000_u32, 500_000, 240_000, 100_000, frequency] {
            if value > 0 && !direct_frequencies.contains(&value) {
                direct_frequencies.push(value);
            }
        }
        for direct_frequency in direct_frequencies {
            let output = run_pyocd_internal_flash_under_reset(
                app,
                target,
                direct_bank,
                &source,
                direct_frequency,
            )?;
            let text = output_text(&output);
            if output.status.success() {
                return Ok(FirmwareWriteResult {
                    summary: format!("{target} flash completed successfully (pyocd under-reset, freq {direct_frequency})"),
                    path: source.to_string_lossy().to_string(),
                    target: target.to_string(),
                    backend: "pyocd".to_string(),
                    frequency: direct_frequency,
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
            emit_firmware_write_progress(
                app,
                target,
                "write",
                0.0,
                0.0,
                direct_frequency,
                "pyocd",
                format!("Under-reset flash failed, trying fallback: {}", text.lines().last().unwrap_or("unknown error")),
            );
        }
    }

    let frequency_attempts = build_backend_frequency_attempts(&used_backend, frequency);
    let mut last_text = String::new();

    for candidate_frequency in frequency_attempts {
        let selected_frequency = candidate_frequency;
        let output = run_flash_command(&used_backend, selected_frequency, target, &source)?;
        let text = output_text(&output);
        if output.status.success() {
            return Ok(FirmwareWriteResult {
                summary: format!("{target} flash completed successfully ({used_backend}, freq {selected_frequency})"),
                path: source.to_string_lossy().to_string(),
                target: target.to_string(),
                backend: used_backend,
                frequency: selected_frequency,
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        last_text = text;
    }

    Err(format!(
        "failed to flash {target}: {}",
        last_text.lines().last().unwrap_or("unknown error")
    ))
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

#[tauri::command]
fn runtime_status() -> RuntimeStatus {
    RuntimeStatus {
      workspace_root: workspace_root().to_string_lossy().to_string(),
      logs_dir: workspace_root().join("logs").to_string_lossy().to_string(),
      tools_dir: runtime_tools_dir().to_string_lossy().to_string(),
      thumbnails_dir: thumbnails_dir().to_string_lossy().to_string(),
      host_root: host_root().to_string_lossy().to_string(),
      gnwmanager_source: if gnwmanager_argv().len() == 1 { "bundled".to_string() } else { "python-module".to_string() },
      rust_backend: "active",
    }
}

#[tauri::command]
fn app_sha256() -> Result<String, String> {
    let exe_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current exe path: {error}"))?;
    let bytes = fs::read(&exe_path)
        .map_err(|error| format!("failed to read current exe {}: {error}", exe_path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn ps_single_quote_text(value: &str) -> String {
    value.replace('\'', "''")
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
fn open_external_url(request: OpenExternalUrlRequest) -> Result<(), String> {
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
        return Ok(());
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
async fn install_app_update(
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

#[tauri::command]
fn load_thumbnail_cache(request: ThumbnailCacheRequest) -> Result<Option<String>, String> {
    let path = thumbnail_cache_path(&request.emulator, &request.title);
    let source_path = if path.exists() {
        path
    } else {
        let legacy_path = legacy_thumbnail_cache_path(&request.emulator, &request.title);
        if !legacy_path.exists() {
            return Ok(None);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("failed to create thumbnail cache dir: {error}"))?;
        }
        let _ = fs::copy(&legacy_path, &path);
        if path.exists() { path } else { legacy_path }
    };
    if !source_path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&source_path).map_err(|error| format!("failed to read thumbnail cache: {error}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:image/png;base64,{encoded}")))
}

#[tauri::command]
fn save_thumbnail_cache(request: ThumbnailSaveRequest) -> Result<String, String> {
    let path = thumbnail_cache_path(&request.emulator, &request.title);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("failed to create thumbnail cache dir: {error}"))?;
    }
    let normalized_png = normalize_thumbnail_png(&request.bytes)?;
    fs::write(&path, normalized_png).map_err(|error| format!("failed to write thumbnail cache: {error}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn read_binary_file(request: BinaryFilePathRequest) -> Result<Vec<u8>, String> {
    let path = PathBuf::from(&request.path);
    let metadata = fs::metadata(&path).map_err(|error| format!("failed to stat file: {error}"))?;
    if !metadata.is_file() {
        return Err("path is not a file".to_string());
    }
    fs::read(&path).map_err(|error| format!("failed to read file: {error}"))
}

#[tauri::command]
fn reveal_path_in_explorer(request: RevealPathRequest) -> Result<(), String> {
    let path = PathBuf::from(&request.path);
    let target = if path.exists() {
        path
    } else {
        return Err("path does not exist".to_string());
    };

    let mut command = Command::new("explorer.exe");
    if target.is_file() {
        command.arg("/select,").arg(&target);
    } else {
        command.arg(&target);
    }

    command
        .spawn()
        .map_err(|error| format!("failed to open explorer: {error}"))?;
    Ok(())
}

#[tauri::command]
fn select_bin_file(request: BinFilePickerRequest) -> Result<Option<BinFilePickerResult>, String> {
    if let Ok(result) = select_bin_file_native(&request.title, request.default_path.as_deref()) {
        return Ok(result);
    }

    select_bin_file_powershell(&request.title, request.default_path.as_deref())
}

#[cfg(target_os = "windows")]
fn select_bin_file_native(title: &str, default_path: Option<&str>) -> Result<Option<BinFilePickerResult>, String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
    };

    struct ComApartment {
        initialized: bool,
    }

    impl Drop for ComApartment {
        fn drop(&mut self) {
            if self.initialized {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let apartment = if init.is_ok() {
            ComApartment { initialized: true }
        } else if init == RPC_E_CHANGED_MODE {
            ComApartment { initialized: false }
        } else {
            return Err(format!("failed to initialize COM: {init:?}"));
        };

        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("failed to create native file picker: {error}"))?;
        let title = HSTRING::from(title);
        dialog
            .SetTitle(PCWSTR(title.as_ptr()))
            .map_err(|error| format!("failed to set file picker title: {error}"))?;

        let filters = [
            COMDLG_FILTERSPEC {
                pszName: w!("Binary files (*.bin)"),
                pszSpec: w!("*.bin"),
            },
            COMDLG_FILTERSPEC {
                pszName: w!("All files (*.*)"),
                pszSpec: w!("*.*"),
            },
        ];
        dialog
            .SetFileTypes(&filters)
            .map_err(|error| format!("failed to set file picker filters: {error}"))?;
        dialog
            .SetFileTypeIndex(1)
            .map_err(|error| format!("failed to set file picker filter index: {error}"))?;

        if let Some(default_path) = default_path.and_then(resolve_file_picker_folder) {
            let folder = HSTRING::from(default_path.to_string_lossy().as_ref());
            if let Ok(shell_item) = SHCreateItemFromParsingName::<_, _, IShellItem>(
                PCWSTR(folder.as_ptr()),
                None,
            ) {
                let _ = dialog.SetFolder(&shell_item);
                let _ = dialog.SetDefaultFolder(&shell_item);
            }
        }

        if let Err(error) = dialog.Show(None) {
            let _ = apartment;
            let code = error.code().0 as u32;
            if code == 0x800704C7 {
                return Ok(None);
            }
            return Err(format!("native file picker failed: {error}"));
        }

        let item = dialog
            .GetResult()
            .map_err(|error| format!("failed to read selected file: {error}"))?;
        let path_ptr = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|error| format!("failed to read selected file path: {error}"))?;
        let path_text = path_ptr
            .to_string()
            .map_err(|error| format!("failed to decode selected file path: {error}"))?;
        CoTaskMemFree(Some(path_ptr.as_ptr().cast()));

        if path_text.trim().is_empty() {
            return Ok(None);
        }
        let path = PathBuf::from(&path_text);
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&path_text)
            .to_string();
        Ok(Some(BinFilePickerResult {
            name,
            path: path_text,
        }))
    }
}

#[cfg(not(target_os = "windows"))]
fn select_bin_file_native(_title: &str, _default_path: Option<&str>) -> Result<Option<BinFilePickerResult>, String> {
    Err("native file picker is only implemented on Windows".to_string())
}

fn resolve_file_picker_folder(default_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(default_path.trim());
    if path.is_dir() {
        return Some(path);
    }
    if path.is_file() {
        return path.parent().map(Path::to_path_buf);
    }
    path.parent()
        .filter(|parent| parent.is_dir())
        .map(Path::to_path_buf)
        .or_else(|| {
            if path.exists() {
                None
            } else if default_path.trim().ends_with('\\') || default_path.trim().ends_with('/') {
                None
            } else {
                path.parent().map(Path::to_path_buf)
            }
        })
}

fn select_bin_file_powershell(title: &str, default_path: Option<&str>) -> Result<Option<BinFilePickerResult>, String> {
    let title = title.replace('\'', " ");
    let initial_dir = default_path
        .and_then(resolve_file_picker_folder)
        .map(|path| path.to_string_lossy().replace('\'', " "));
    let initial_dir_script = initial_dir
        .as_ref()
        .map(|path| format!("$dialog.InitialDirectory = '{}'; ", path))
        .unwrap_or_default();
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $dialog = New-Object System.Windows.Forms.OpenFileDialog; \
         $dialog.Title = '{}'; \
         {}\
         $dialog.Filter = 'Binary files (*.bin)|*.bin|All files (*.*)|*.*'; \
         $dialog.CheckFileExists = $true; \
         $dialog.Multiselect = $false; \
         if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::Out.Write($dialog.FileName) }}",
        title,
        initial_dir_script
    );
    let mut command = Command::new("powershell");
    command
        .arg("-NoProfile")
        .arg("-STA")
        .arg("-Command")
        .arg(script);
    let output = hide_command_window(&mut command)
        .output()
        .map_err(|error| format!("failed to open file picker: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "file picker failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let path_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path_text.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(&path_text);
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&path_text)
        .to_string();
    Ok(Some(BinFilePickerResult {
        name,
        path: path_text,
    }))
}

#[tauri::command]
fn import_rom_files(request: RomImportRequest) -> Result<RomImportResult, String> {
    let extensions = rom_extensions_for_emulator(&request.emulator);
    if extensions.is_empty() {
        return Err(format!("unsupported emulator: {}", request.emulator));
    }

    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for file in request.files {
        let lower_name = file.name.to_ascii_lowercase();
        let source_label = std::path::Path::new(&file.name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("import");

        if extensions.iter().any(|ext| lower_name.ends_with(ext)) {
            let dest_path = write_imported_rom(&request.emulator, source_label, &file.name, &file.bytes)?;
            let title = std::path::Path::new(&file.name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or(&file.name)
                .to_string();
            entries.push(RomImportEntry {
                title,
                path: dest_path.to_string_lossy().to_string(),
                size_bytes: file.bytes.len() as u64,
            });
            continue;
        }

        if lower_name.ends_with(".zip") {
            let reader = Cursor::new(file.bytes);
            let mut archive = zip::ZipArchive::new(reader)
                .map_err(|error| format!("failed to open zip archive {}: {error}", file.name))?;

            for index in 0..archive.len() {
                let mut item = archive
                    .by_index(index)
                    .map_err(|error| format!("failed to read zip entry in {}: {error}", file.name))?;
                if item.is_dir() {
                    continue;
                }
                let entry_name = item.name().replace('\\', "/");
                let entry_lower = entry_name.to_ascii_lowercase();
                if !extensions.iter().any(|ext| entry_lower.ends_with(ext)) {
                    continue;
                }
                let base_name = std::path::Path::new(&entry_name)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&entry_name)
                    .to_string();
                let mut content = Vec::new();
                item.read_to_end(&mut content)
                    .map_err(|error| format!("failed to extract zip entry {entry_name}: {error}"))?;
                let dest_path = write_imported_rom(&request.emulator, source_label, &base_name, &content)?;
                let title = std::path::Path::new(&base_name)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&base_name)
                    .to_string();
                entries.push(RomImportEntry {
                    title,
                    path: dest_path.to_string_lossy().to_string(),
                    size_bytes: content.len() as u64,
                });
            }
            continue;
        }

        if lower_name.ends_with(".7z") {
            warnings.push(format!(
                "7z archive is not supported yet: {}. Repack to ZIP or extract manually.",
                file.name
            ));
            continue;
        }

        warnings.push(format!("Unsupported file skipped: {}", file.name));
    }

    Ok(RomImportResult { entries, warnings })
}

#[tauri::command]
fn import_rom_files_auto(files: Vec<RomImportFile>) -> Result<AutoRomImportResult, String> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for file in files {
        import_auto_from_named_bytes(&file.name, file.bytes, &mut entries, &mut warnings)?;
    }

    Ok(AutoRomImportResult { entries, warnings })
}

#[tauri::command]
fn import_rom_paths_auto(request: RomImportPathsRequest) -> Result<AutoRomImportResult, String> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for path in request.paths {
        let path_buf = PathBuf::from(&path);
        let file_name = path_buf
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&path)
            .to_string();
        if !path_buf.is_file() {
            warnings.push(format!("Dropped path ignored (not a file): {}", path));
            continue;
        }
        if !is_supported_import_name(&file_name) {
            warnings.push(format!("Dropped file ignored: {}", file_name));
            continue;
        }
        let bytes = fs::read(&path).map_err(|error| format!("failed to read dropped file {}: {error}", path))?;
        import_auto_from_named_bytes(&file_name, bytes, &mut entries, &mut warnings)?;
    }

    Ok(AutoRomImportResult { entries, warnings })
}

#[tauri::command]
fn check_msx_bios() -> MsxBiosStatus {
    msx_bios_status()
}

#[tauri::command]
fn save_msx_bios_paths(request: MsxBiosPathsRequest) -> Result<MsxBiosStatus, String> {
    for path in request.paths {
        let path_buf = PathBuf::from(&path);
        if !path_buf.is_file() {
            continue;
        }
        let file_name = path_buf
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let bytes = fs::read(&path_buf)
            .map_err(|error| format!("failed to read MSX BIOS candidate {}: {error}", path_buf.display()))?;
        let _ = save_msx_bios_file(&file_name, &bytes)?;
    }
    Ok(msx_bios_status())
}

#[tauri::command]
fn save_msx_bios_files(request: MsxBiosFilesRequest) -> Result<MsxBiosStatus, String> {
    for file in request.files {
        let _ = save_msx_bios_file(&file.name, &file.bytes)?;
    }
    Ok(msx_bios_status())
}

#[tauri::command]
fn check_coleco_bios() -> MsxBiosStatus {
    coleco_bios_status()
}

#[tauri::command]
fn save_coleco_bios_paths(request: MsxBiosPathsRequest) -> Result<MsxBiosStatus, String> {
    let mut last_error = None;
    for path in request.paths {
        let path_buf = PathBuf::from(&path);
        if !path_buf.is_file() {
            continue;
        }
        let file_name = path_buf
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(COLECO_BIOS_FILE_NAME)
            .to_string();
        let bytes = fs::read(&path_buf)
            .map_err(|error| format!("failed to read ColecoVision BIOS candidate {}: {error}", path_buf.display()))?;
        match save_coleco_bios_file(&file_name, &bytes) {
            Ok(true) => return Ok(coleco_bios_status()),
            Ok(false) => {}
            Err(error) => last_error = Some(error),
        }
    }
    if coleco_bios_status().ready {
        Ok(coleco_bios_status())
    } else {
        Err(last_error.unwrap_or_else(|| format!("drop a valid {}", COLECO_BIOS_FILE_NAME)))
    }
}

#[tauri::command]
fn save_coleco_bios_files(request: MsxBiosFilesRequest) -> Result<MsxBiosStatus, String> {
    let mut last_error = None;
    for file in request.files {
        match save_coleco_bios_file(&file.name, &file.bytes) {
            Ok(true) => return Ok(coleco_bios_status()),
            Ok(false) => {}
            Err(error) => last_error = Some(error),
        }
    }
    if coleco_bios_status().ready {
        Ok(coleco_bios_status())
    } else {
        Err(last_error.unwrap_or_else(|| format!("drop a valid {}", COLECO_BIOS_FILE_NAME)))
    }
}

#[tauri::command]
async fn read_device_info(_app: tauri::AppHandle, backend: String, frequency: u32) -> Result<DeviceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _requested_backend = backend;
        let used_backend = "pyocd".to_string();
        let (text, used_frequency, info_source) = match run_gnwmanager_info_with_frequency_fallback(&used_backend, frequency) {
            Ok((output, used_frequency)) => (output_text(&output), used_frequency, "gnwmanager"),
            Err(gnwmanager_error) => {
                let (text, used_frequency) = run_direct_info_with_frequency_fallback(frequency)
                    .map_err(|direct_error| format!("{gnwmanager_error}; {direct_error}"))?;
                (text, used_frequency, "direct under-reset")
            }
        };

        let details = parse_details(&text);
        let programmer = detail(&details, &["Programmer", "Debug Probe"])
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let probe_vendor = detail(&details, &["Probe vendor", "Probe Vendor"]).unwrap_or_else(|| {
            if programmer.to_ascii_lowercase().contains("stlink") || programmer.to_ascii_lowercase().contains("st-link") {
                "STMicroelectronics".to_string()
            } else {
                "UNKNOWN".to_string()
            }
        });
        let probe_id = detail(&details, &["Probe ID", "Probe Id", "Probe UID"]).unwrap_or_else(|| {
            if programmer != "UNKNOWN" {
                "NOT EXPOSED".to_string()
            } else {
                "UNKNOWN".to_string()
            }
        });
        let mcu_profile = detail(&details, &["Target type", "MCU", "MCU profile", "MCU Profile"])
            .unwrap_or_else(|| "STM32H7B0xx".to_string());
        let cpu_id = detail(
            &details,
            &[
                "CPU ID",
                "CPUID",
                "CPU",
                "Device ID",
                "DBGMCU IDCODE",
                "IDCODE",
                "Target ID",
                "Part number",
                "Part Number",
            ],
        )
        .unwrap_or_else(|| "UNKNOWN".to_string());
        let target_voltage_from_info = detail(
            &details,
            &[
                "Target voltage",
                "Target Voltage",
                "Target voltage (V)",
                "VTarget",
                "VTref",
                "Voltage",
            ],
        )
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                "UNKNOWN".to_string()
            } else if trimmed.to_ascii_lowercase().contains('v') {
                trimmed.to_string()
            } else {
                format!("{trimmed} V")
            }
        })
        .unwrap_or_else(|| "UNKNOWN".to_string());
        let target_voltage = if target_voltage_from_info.trim().eq_ignore_ascii_case("UNKNOWN") {
            read_target_voltage_pyocd(used_frequency).unwrap_or(target_voltage_from_info)
        } else {
            target_voltage_from_info
        };
        let external_flash = detail(
            &details,
            &["External flash", "Extflash size", "External Flash Size (MB)"],
        )
        .map(|value| {
            if details.contains_key("External Flash Size (MB)") && !value.to_ascii_lowercase().contains("mb") {
                format!("{value} MB")
            } else {
                value
            }
        })
        .unwrap_or_else(|| "UNKNOWN".to_string());
        let device_uid = device_uid_from_details(&details, &text)
            .or_else(|| {
                read_info_frequency_attempts(used_frequency)
                    .into_iter()
                    .find_map(|candidate_frequency| read_device_uid_under_reset(candidate_frequency).ok())
            })
            .unwrap_or_else(|| "UNKNOWN".to_string());
        if device_uid != "UNKNOWN" {
            set_current_device_uid(Some(device_uid.clone()));
            fs::create_dir_all(device_backups_dir(&device_uid))
                .map_err(|error| format!("failed to create device backup dir: {error}"))?;
            thread::sleep(Duration::from_millis(250));
        } else {
            set_current_device_uid(None);
        }
        let summary = format!("Device info read successfully ({info_source}, {used_backend}, freq {used_frequency})");
        let detected_firmware = detail(
            &details,
            &["Stock firmware", "Detected firmware", "Detected Stock Firmware"],
        )
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let protection = detail(&details, &["Locked?", "Protection"])
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let filesystem = detail(&details, &["Filesystem", "Filesystem Size (B)"])
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let device_info = DeviceInfo {
            summary,
            programmer,
            probe_vendor,
            probe_id,
            device_uid,
            cpu_id,
            target_voltage,
            mcu_profile,
            detected_firmware,
            external_flash,
            protection,
            filesystem,
        };
        update_service_bridge_state(&device_info.summary, Some(&device_info));
        Ok(device_info)
    })
    .await
    .map_err(|error| format!("failed to join device info task: {error}"))?
}

#[tauri::command]
fn lookup_device_backups(request: DeviceBackupLookupRequest) -> Result<DeviceBackupLookupResult, String> {
    let uid = request.device_uid.trim();
    if uid.is_empty() || uid.eq_ignore_ascii_case("UNKNOWN") {
        return Ok(DeviceBackupLookupResult {
            mcu_name: String::new(),
            mcu_path: String::new(),
            bank2_name: String::new(),
            bank2_path: String::new(),
            spi_name: String::new(),
            spi_path: String::new(),
        });
    }

    let dir = device_backups_dir(uid);
    let legacy_dir = legacy_device_backups_dir(uid);
    let mcu = newest_restore_bank1_file(&dir, &legacy_dir);
    let bank2 = newest_restore_bank2_file(&dir, &legacy_dir);
    let spi = newest_restore_spi_file(&dir, &legacy_dir);

    Ok(DeviceBackupLookupResult {
        mcu_name: mcu
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        mcu_path: mcu
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        bank2_name: bank2
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        bank2_path: bank2
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        spi_name: spi
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        spi_path: spi
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    })
}

#[tauri::command]
fn lookup_stock_backups(request: DeviceBackupLookupRequest) -> Result<DeviceBackupLookupResult, String> {
    let dir = stock_firmware_dir();
    let legacy_dir = dir.clone();
    let profile_code = request
        .firmware_profile
        .as_deref()
        .map(firmware_profile_code)
        .unwrap_or("m");
    let mcu = newest_valid_stock_bank1_file(&dir, &legacy_dir, profile_code);
    let bank2: Option<PathBuf> = None;
    let spi = newest_valid_stock_spi_file(&dir, &legacy_dir, profile_code);

    Ok(DeviceBackupLookupResult {
        mcu_name: mcu
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        mcu_path: mcu
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        bank2_name: bank2
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        bank2_path: bank2
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        spi_name: spi
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        spi_path: spi
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    })
}

#[tauri::command]
fn lookup_restore_backups(request: DeviceBackupLookupRequest) -> Result<DeviceBackupLookupResult, String> {
    let uid = request.device_uid.trim();
    if uid.is_empty() || uid.eq_ignore_ascii_case("UNKNOWN") {
        return Ok(DeviceBackupLookupResult {
            mcu_name: String::new(),
            mcu_path: String::new(),
            bank2_name: String::new(),
            bank2_path: String::new(),
            spi_name: String::new(),
            spi_path: String::new(),
        });
    }

    let dir = device_backups_dir(uid);
    let legacy_dir = legacy_device_backups_dir(uid);
    let mcu = newest_restore_bank1_file(&dir, &legacy_dir);
    let bank2 = newest_restore_bank2_file(&dir, &legacy_dir);
    let spi = newest_restore_spi_file(&dir, &legacy_dir);

    Ok(DeviceBackupLookupResult {
        mcu_name: mcu
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        mcu_path: mcu
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        bank2_name: bank2
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        bank2_path: bank2
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        spi_name: spi
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        spi_path: spi
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    })
}

#[tauri::command]
fn import_stock_backup(request: StockBackupImportRequest) -> Result<StockBackupImportResult, String> {
    let requested_model_name = resolve_model_name(&request.firmware_profile)?;
    let kind = request.kind.trim().to_ascii_lowercase();
    let part = match kind.as_str() {
        "mcu" | "bank1" => "bank1",
        "spi" | "ext" | "extflash" => "spi",
        _ => return Err(format!("unsupported stock backup kind: {}", request.kind)),
    };

    let source = PathBuf::from(request.path.trim());
    if !source.exists() {
        return Err(format!("stock backup file not found: {}", source.display()));
    }
    if !source.is_file() {
        return Err(format!("stock backup path is not a file: {}", source.display()));
    }

    let size_bytes = fs::metadata(&source)
        .map_err(|error| format!("failed to stat stock backup file: {error}"))?
        .len();
    let detected_model_name = match part {
        "bank1" => detect_stock_bank1_profile(&source)?,
        "spi" => detect_stock_spi_profile(&source)?,
        _ => requested_model_name,
    };
    if detected_model_name != requested_model_name {
        return Err(format!(
            "Selected {} stock firmware is for {}, but current operation requires {}. Choose matching original {} stock files.",
            if part == "bank1" { "Bank1" } else { "SPI" },
            profile_display_name(detected_model_name),
            profile_display_name(requested_model_name),
            profile_display_name(requested_model_name)
        ));
    }
    match part {
        "bank1" => validate_stock_bank1_file(&source, requested_model_name)?,
        "spi" => validate_stock_spi_file(&source, requested_model_name)?,
        _ => {}
    }
    validate_stock_backup_size(part, detected_model_name, size_bytes)?;

    let stock_dir = stock_firmware_dir();
    fs::create_dir_all(&stock_dir)
        .map_err(|error| format!("failed to create stock firmware dir: {error}"))?;
    let destination = stock_dir.join(stock_firmware_output_name(part, detected_model_name));
    fs::copy(&source, &destination)
        .map_err(|error| format!("failed to import stock backup: {error}"))?;

    Ok(StockBackupImportResult {
        name: destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        path: destination.to_string_lossy().to_string(),
        size_bytes,
    })
}

fn validate_stock_backup_size(part: &str, model_name: &str, size_bytes: u64) -> Result<(), String> {
    match part {
        "bank1" => {
            let min = 128 * 1024;
            let max = 1024 * 1024;
            if size_bytes < min || size_bytes > max {
                return Err(format!(
                    "Bank1 backup size must be between 128 KB and 1 MB, got {} bytes",
                    size_bytes
                ));
            }
        }
        "spi" => {
            let min = if model_name == "z" { 4 } else { 1 } * 1024 * 1024;
            if size_bytes < min {
                return Err(format!(
                    "SPI backup is too small for {} profile: got {} bytes, need at least {} bytes",
                    model_name.to_ascii_uppercase(),
                    size_bytes,
                    min
                ));
            }
        }
        _ => return Err(format!("unsupported stock backup part: {part}")),
    }
    Ok(())
}

fn resolve_model_name(model: &str) -> Result<&'static str, String> {
    let normalized_model = model.trim().to_ascii_lowercase();
    if normalized_model == "m" {
        Ok("m")
    } else if normalized_model == "z" {
        Ok("z")
    } else {
        Err(format!("unsupported backup model: {model}"))
    }
}

fn profile_display_name(model_name: &str) -> &'static str {
    if model_name == "z" {
        "Zelda"
    } else {
        "Mario"
    }
}

fn mirror_backup_to_stock_name(source: &Path, part: &str, model_name: &str) -> Result<(), String> {
    let detected_model_name = match part {
        "bank1" => match detect_stock_bank1_profile(source) {
            Ok(profile) => profile,
            Err(_) => return Ok(()),
        },
        "spi" => match detect_stock_spi_profile(source) {
            Ok(profile) => profile,
            Err(_) => return Ok(()),
        },
        _ => model_name,
    };

    let destination_dir = stock_firmware_dir();
    fs::create_dir_all(&destination_dir)
        .map_err(|error| format!("failed to create stock firmware dir: {error}"))?;
    let destination = destination_dir.join(stock_firmware_output_name(part, detected_model_name));
    if source != destination {
        fs::copy(source, &destination)
            .map_err(|error| format!("failed to save stock-named backup {}: {error}", destination.display()))?;
    }
    Ok(())
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
    let mut final_output: Option<Output> = None;

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
async fn read_mcu_backup(
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
async fn read_bank2_backup(
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
async fn read_spi_backup(
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

#[tauri::command]
async fn write_bank1_firmware(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    path: String,
) -> Result<FirmwareWriteResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_flash_phase(&app, backend, frequency, protection, path, "bank1", 0.0, 0)
    })
    .await
    .map_err(|error| format!("failed to join bank1 flash task: {error}"))?
}

#[tauri::command]
async fn write_bank2_firmware(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    path: String,
) -> Result<FirmwareWriteResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_flash_phase(&app, backend, frequency, protection, path, "bank2", 0.0, 0)
    })
    .await
    .map_err(|error| format!("failed to join bank2 flash task: {error}"))?
}

#[tauri::command]
async fn write_spi_firmware(
    app: tauri::AppHandle,
    backend: String,
    frequency: u32,
    protection: String,
    path: String,
    external_flash_mb: f64,
    external_flash_offset_bytes: u64,
) -> Result<FirmwareWriteResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_single_flash_phase(&app, backend, frequency, protection, path, "ext", external_flash_mb, external_flash_offset_bytes)
    })
    .await
    .map_err(|error| format!("failed to join SPI flash task: {error}"))?
}

#[tauri::command]
fn builder_placeholder(action: String) -> String {
    format!("Rust backend placeholder for action: {action}")
}

pub fn run() {
    if let Err(error) = validate_exe_path_for_portable_runtime() {
        show_startup_error(&error);
        return;
    }

    let run_result = tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_theme(Some(tauri::Theme::Dark));
            }
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                let event = match prepare_portable_runtime(&app_handle) {
                    Ok(Some(runtime_dir)) => PortableRuntimeReadyEvent {
                        ok: true,
                        runtime_dir: runtime_dir.to_string_lossy().to_string(),
                        message: "portable runtime ready".to_string(),
                    },
                    Ok(None) => PortableRuntimeReadyEvent {
                        ok: true,
                        runtime_dir: String::new(),
                        message: "portable runtime not bundled".to_string(),
                    },
                    Err(error) => PortableRuntimeReadyEvent {
                        ok: false,
                        runtime_dir: String::new(),
                        message: format!("portable runtime extraction failed: {error}"),
                    },
                };
                let _ = app_handle.emit("portable-runtime-ready", event);
            });
            start_service_bridge_listener();
            Ok(())
        })
        .on_window_event(|_window, event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                spawn_runtime_cleanup_helper();
                std::process::exit(0);
            } else if matches!(event, WindowEvent::Destroyed) {
                spawn_runtime_cleanup_helper();
                cleanup_current_portable_runtime_dir();
            }
        })
        .invoke_handler(tauri::generate_handler![
            runtime_status,
            app_sha256,
            open_external_url,
            install_app_update,
            load_thumbnail_cache,
            save_thumbnail_cache,
            read_binary_file,
            reveal_path_in_explorer,
            select_bin_file,
            import_rom_files,
            import_rom_files_auto,
            import_rom_paths_auto,
            check_msx_bios,
            save_msx_bios_paths,
            save_msx_bios_files,
            check_coleco_bios,
            save_coleco_bios_paths,
            save_coleco_bios_files,
            compute_build_metrics,
            build_firmware_bundle,
            latest_firmware_bundle,
            read_device_info,
            lookup_device_backups,
            lookup_restore_backups,
            lookup_stock_backups,
            import_stock_backup,
            read_mcu_backup,
            read_bank2_backup,
            read_spi_backup,
            write_bank1_firmware,
            write_bank2_firmware,
            write_spi_firmware,
            builder_placeholder
        ])
        .run(tauri::generate_context!());

    cleanup_current_portable_runtime_dir();

    run_result.expect("error while running tauri application");
}
