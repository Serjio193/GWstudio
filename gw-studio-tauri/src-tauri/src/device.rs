use crate::backups::device_backups_dir;
use crate::service_bridge::update_service_bridge_state;
use crate::device_state::set_current_device_uid;
use crate::toolchain::locate_python_exe;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::{Command, Output};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize)]
pub(crate) struct DeviceInfo {
    pub(crate) summary: String,
    pub(crate) programmer: String,
    pub(crate) probe_vendor: String,
    pub(crate) probe_id: String,
    pub(crate) device_uid: String,
    pub(crate) cpu_id: String,
    pub(crate) target_voltage: String,
    pub(crate) mcu_profile: String,
    pub(crate) detected_firmware: String,
    pub(crate) external_flash: String,
    pub(crate) protection: String,
    pub(crate) filesystem: String,
}

fn hide_command_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.trim().is_empty() {
        stdout.to_string()
    } else if stdout.trim().is_empty() {
        stderr.to_string()
    } else {
        format!("{stdout}\n{stderr}")
    }
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

pub(crate) fn gnwmanager_argv() -> Vec<String> {
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

fn detail(details: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
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
        .arg("info");

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
        1_000_000,
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
        match run_gnwmanager_info_with_retries(backend, candidate_frequency, 1) {
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

#[tauri::command]
pub(crate) async fn read_device_info(_app: tauri::AppHandle, backend: String, frequency: u32) -> Result<DeviceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _requested_backend = backend;
        let used_backend = "pyocd".to_string();
        let (text, used_frequency, info_source) = match run_direct_info_with_frequency_fallback(frequency) {
            Ok((text, used_frequency)) => (text, used_frequency, "direct under-reset"),
            Err(direct_error) => {
                let (output, used_frequency) = run_gnwmanager_info_with_frequency_fallback(&used_backend, frequency)
                    .map_err(|gnwmanager_error| format!("{direct_error}; {gnwmanager_error}"))?;
                (output_text(&output), used_frequency, "gnwmanager")
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
