use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::bios::{copy_coleco_bios_to_workspace, copy_msx_bios_to_workspace};
use crate::build_events::emit_build_progress;
use crate::build_images::{emulator_icon_path, write_coverflow_source_image};
use crate::build_workspace::{
    build_workspaces_dir, bundles_dir, copy_dir_filtered, copy_if_exists, ensure_clean_dir,
    retro_go_emulator_dir, retro_go_workspace_rom_name, sanitize_file_part,
};
use crate::firmware_image::compose_spi_image;
use crate::game_watch_patch::run_game_watch_patch_build;
use crate::retro_go_build::run_retro_go_build;
use crate::retro_go_patch::patch_retro_go_workspace_for_windows;
use crate::retro_go_source::locate_retro_go_repo;
use crate::stock::{
    bundle_stamp, explicit_stock_file, firmware_output_name, firmware_profile_code,
};
use crate::thumbnails::{sanitize_thumbnail_part, thumbnail_cache_path};

#[derive(Deserialize)]
pub(crate) struct BuildBundleEntry {
    emulator: String,
    title: String,
    rom_path: String,
}

#[derive(Deserialize)]
pub(crate) struct BuildBundleRequest {
    firmware_profile: String,
    dual_boot: Option<bool>,
    installed_spi_mb: f64,
    firmware_reserved_mb: f64,
    stock_bank1_path: Option<String>,
    stock_spi_path: Option<String>,
    coverflow_enabled: Option<bool>,
    entries: Vec<BuildBundleEntry>,
}

#[derive(Serialize)]
pub(crate) struct BuildBundleResult {
    bundle_dir: String,
    dual_boot: bool,
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

#[tauri::command]
pub(crate) async fn build_firmware_bundle(
    app: tauri::AppHandle,
    request: BuildBundleRequest,
) -> Result<BuildBundleResult, String> {
    tauri::async_runtime::spawn_blocking(move || build_firmware_bundle_blocking(&app, request))
        .await
        .map_err(|error| format!("failed to join build firmware task: {error}"))?
}

fn build_firmware_bundle_blocking(
    app: &tauri::AppHandle,
    request: BuildBundleRequest,
) -> Result<BuildBundleResult, String> {
    if request.entries.is_empty() {
        return Err("no ROM entries provided for bundle".to_string());
    }

    emit_build_progress(app, 2.0, "Preparing build workspace");

    let coverflow_enabled = request.coverflow_enabled.unwrap_or(false);
    let dual_boot = request.dual_boot.unwrap_or(true);
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
    let flash_notes_path = bundle_dir.join("flash_bundle.txt");
    let stock_bank1_candidate = if dual_boot {
        Some(explicit_stock_file(request.stock_bank1_path.as_deref())?)
    } else {
        None
    };
    let stock_spi_candidate = if dual_boot {
        Some(explicit_stock_file(request.stock_spi_path.as_deref())?)
    } else {
        None
    };
    let extflash_build_path = retro_go_workspace_dir
        .join("build")
        .join("gw_retro_go_extflash.bin");
    let intflash_build_path = retro_go_workspace_dir
        .join("build")
        .join("gw_retro_go_intflash.bin");
    let build_log_path = retro_go_workspace_dir.join("build_gw_studio.log");
    let patch_workspace_dir = build_workspaces_dir().join(format!("game_watch_patch_{}", bundle_name));
    let extflash_offset_bytes = if dual_boot { firmware_reserved_bytes } else { 0 };
    let reserved_spi_mb = if dual_boot {
        request.firmware_reserved_mb
    } else {
        0.0
    };
    let extflash_size_mb = (request.installed_spi_mb - reserved_spi_mb)
        .max(1.0)
        .round() as u32;
    let patched_bank1_candidate = if dual_boot {
        Some(run_game_watch_patch_build(
            app,
            &patch_workspace_dir,
            stock_bank1_candidate
                .as_deref()
                .ok_or_else(|| "stock Bank1 path is missing".to_string())?,
            stock_spi_candidate
                .as_deref()
                .ok_or_else(|| "stock SPI path is missing".to_string())?,
            &firmware_profile,
        )?)
    } else {
        None
    };
    emit_build_progress(app, 64.0, "Workspace ready, starting Retro-Go fork build");
    let _actual_build_log_path = run_retro_go_build(
        app,
        &retro_go_workspace_dir,
        extflash_size_mb,
        extflash_offset_bytes,
        if dual_boot { 2 } else { 1 },
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
    let bank1_source = if dual_boot {
        patched_bank1_candidate.as_ref()
    } else {
        Some(&intflash_build_path)
    };
    let bank1_output = if let Some(source) = bank1_source {
        if copy_if_exists(source, &bank1_output_path)? {
            Some(bank1_output_path.clone())
        } else {
            None
        }
    } else {
        None
    };
    let bank2_output = if dual_boot && copy_if_exists(&intflash_build_path, &bank2_output_path)? {
        Some(bank2_output_path.clone())
    } else {
        None
    };
    let patched_spi_candidate = patch_workspace_dir.join("build").join("external_flash_patched.bin");
    let patched_spi_size = fs::metadata(&patched_spi_candidate).map(|metadata| metadata.len()).unwrap_or(0);
    let spi_prefix_source = if !dual_boot {
        None
    } else if patched_spi_size >= extflash_offset_bytes {
        Some(patched_spi_candidate.clone())
    } else {
        stock_spi_candidate.clone()
    };
    if let Some(prefix_source) = spi_prefix_source.as_deref() {
        compose_spi_image(prefix_source, &extflash_build_path, &spi_output_path, extflash_offset_bytes)?;
    } else {
        fs::copy(&extflash_build_path, &spi_output_path)
            .map_err(|error| format!("failed to copy standalone SPI image: {error}"))?;
    }
    let spi_output = spi_output_path.clone();
    let spi_output_size_bytes = fs::metadata(&spi_output).map(|metadata| metadata.len()).unwrap_or(0);

    let mut summary_lines = vec![
        format!("Bundle: {bundle_name}"),
        format!("Firmware profile: {}", request.firmware_profile),
        format!("Dual boot: {}", dual_boot),
        format!("Firmware code: {}", firmware_profile.to_ascii_uppercase()),
        format!("Installed SPI MB: {}", request.installed_spi_mb),
        format!("Firmware reserved MB: {}", request.firmware_reserved_mb),
        format!("Retro-Go fork EXTFLASH_SIZE_MB: {}", extflash_size_mb),
        format!("Retro-Go fork EXTFLASH_OFFSET: {}", extflash_offset_bytes),
        format!("Retro-Go fork INTFLASH_BANK: {}", if dual_boot { 2 } else { 1 }),
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
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "not used".to_string())
        ),
        format!(
            "Stock SPI source: {}",
            stock_spi_candidate
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "not used".to_string())
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
        format!(
            "SPI prefix source: {}",
            spi_prefix_source
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string())
        ),
        format!("Retro-Go fork extflash payload size: {}", extflash_build_size_bytes),
        format!("Flashable SPI image size: {}", spi_output_size_bytes),
        if dual_boot {
            "Flashable SPI image includes stock SPI prefix".to_string()
        } else {
            "Flashable SPI image contains Retro-Go only".to_string()
        },
        String::new(),
        "Emulators:".to_string(),
    ];
    for (emulator, count) in grouped_counts {
        summary_lines.push(format!("- {emulator}: {count} rom(s)"));
    }
    fs::write(&summary_path, summary_lines.join("\n")).map_err(|error| format!("failed to write bundle summary: {error}"))?;

    let mut firmware_artifacts = Vec::new();
    if let Some(path) = bank1_output.as_ref() {
        firmware_artifacts.push(firmware_artifact_json("bank1", &firmware_profile, path, Some(0))?);
    }
    if let Some(path) = bank2_output.as_ref() {
        firmware_artifacts.push(firmware_artifact_json("bank2", &firmware_profile, path, Some(0))?);
    }
    firmware_artifacts.push(firmware_artifact_json(
        "spi",
        &firmware_profile,
        &spi_output,
        Some(0),
    )?);

    let manifest = serde_json::json!({
        "manifest_version": 2,
        "bundle_name": bundle_name,
        "firmware_profile": request.firmware_profile,
        "firmware_code": firmware_profile,
        "dual_boot": dual_boot,
        "installed_spi_mb": request.installed_spi_mb,
        "firmware_reserved_mb": request.firmware_reserved_mb,
        "retro_go_extflash_size_mb": extflash_size_mb,
        "retro_go_extflash_offset_bytes": extflash_offset_bytes,
        "retro_go_intflash_bank": if dual_boot { 2 } else { 1 },
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
        "bank1_dualboot": dual_boot,
        "bank1_dualboot_entry": if dual_boot { Some("LEFT+GAME") } else { None },
        "bank2_candidate_path": bank2_output,
        "extflash_build_path": spi_output,
        "extflash_built": extflash_built,
        "extflash_build_size_bytes": spi_output_size_bytes,
        "retro_go_extflash_payload_size_bytes": extflash_build_size_bytes,
        "spi_full_image": true,
        "firmware_artifacts": firmware_artifacts,
        "files": {
            "summary": summary_path,
            "flash_script": flash_notes_path,
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
        &flash_notes_path,
        [
            "Build bundle prepared by GW Studio",
            &format!("# Retro-Go fork workspace: {}", retro_go_workspace_dir.display()),
            &format!("# Flashable Bank1 image: {}", bank1_output_path.display()),
            &format!("# Flashable Bank2 image: {}", bank2_output_path.display()),
            &format!("# Flashable SPI image: {}", spi_output.display()),
            &format!("# Build log: {}", build_log_path.display()),
            if dual_boot {
                "# Use GW Studio Flash Console to write Bank1 + Bank2 + SPI."
            } else {
                "# Use GW Studio Flash Console to write Retro-Go Bank1 + SPI."
            },
            "",
        ]
        .join("\n"),
    )
    .map_err(|error| format!("failed to write flash script info: {error}"))?;

    Ok(BuildBundleResult {
        bundle_dir: bundle_dir.to_string_lossy().to_string(),
        dual_boot,
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

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read {} for SHA256: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn firmware_artifact_json(
    target: &str,
    firmware_code: &str,
    path: &Path,
    offset_bytes: Option<u64>,
) -> Result<serde_json::Value, String> {
    let size_bytes = fs::metadata(path)
        .map_err(|error| format!("failed to stat firmware artifact {}: {error}", path.display()))?
        .len();
    Ok(serde_json::json!({
        "target": target,
        "firmware_code": firmware_code,
        "path": path,
        "file_name": path.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
        "size_bytes": size_bytes,
        "sha256": sha256_file(path)?,
        "offset_bytes": offset_bytes,
    }))
}
