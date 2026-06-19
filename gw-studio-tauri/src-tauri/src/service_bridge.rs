use crate::paths::host_root;
use crate::backups::backup_ready_for_device;
use crate::device::DeviceInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

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

fn write_service_bridge_snapshot_file() -> Result<(), String> {
    fs::create_dir_all(service_dir())
        .map_err(|error| format!("failed to create service dir: {error}"))?;
    let snapshot = service_bridge_snapshot();
    let path = service_state_path();
    fs::write(&path, service_state_text(&snapshot))
        .map_err(|error| format!("failed to write service snapshot: {error}"))?;
    Ok(())
}

pub(crate) fn update_service_bridge_state(message: &str, device_info: Option<&DeviceInfo>) {
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

pub(crate) fn start_service_bridge_listener() {
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
