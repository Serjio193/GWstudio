use crate::paths::host_root;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct RomImportFile {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
pub(crate) struct MsxBiosPathsRequest {
    paths: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct MsxBiosFilesRequest {
    files: Vec<RomImportFile>,
}

#[derive(Serialize)]
pub(crate) struct MsxBiosStatus {
    ready: bool,
    dir: String,
    missing: Vec<String>,
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

fn msx_bios_store_dir() -> PathBuf {
    host_root().join("msx_bios")
}

fn coleco_bios_store_dir() -> PathBuf {
    host_root().join("coleco_bios")
}

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

pub(crate) fn copy_msx_bios_to_workspace(workspace_dir: &Path) -> Result<(), String> {
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

pub(crate) fn copy_coleco_bios_to_workspace(workspace_dir: &Path) -> Result<(), String> {
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

#[tauri::command]
pub(crate) fn check_msx_bios() -> MsxBiosStatus {
    msx_bios_status()
}

#[tauri::command]
pub(crate) fn save_msx_bios_paths(request: MsxBiosPathsRequest) -> Result<MsxBiosStatus, String> {
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
pub(crate) fn save_msx_bios_files(request: MsxBiosFilesRequest) -> Result<MsxBiosStatus, String> {
    for file in request.files {
        let _ = save_msx_bios_file(&file.name, &file.bytes)?;
    }
    Ok(msx_bios_status())
}

#[tauri::command]
pub(crate) fn check_coleco_bios() -> MsxBiosStatus {
    coleco_bios_status()
}

#[tauri::command]
pub(crate) fn save_coleco_bios_paths(request: MsxBiosPathsRequest) -> Result<MsxBiosStatus, String> {
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
pub(crate) fn save_coleco_bios_files(request: MsxBiosFilesRequest) -> Result<MsxBiosStatus, String> {
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
