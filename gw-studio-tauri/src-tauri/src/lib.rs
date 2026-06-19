use std::thread;
use tauri::{Emitter, Manager};
use tauri::WindowEvent;

mod paths;
mod runtime;
mod service_bridge;
mod updater;
mod file_picker;
mod thumbnails;
mod roms;
mod bios;
mod stock;
mod backups;
mod toolchain;
mod device;
mod backup_read;
mod process_stream;
mod flash;
mod app_status;
mod device_state;
mod process_helpers;
mod bundle_lookup;
mod build_events;
mod build_metrics;
mod build_images;
mod build_workspace;
mod retro_go_patch;
mod retro_go_build;
mod firmware_image;
mod game_watch_patch;
mod retro_go_source;
mod firmware_build;
mod pyocd_transport;
mod gnwmanager_transport;
mod spi_helper;
mod startup;

use paths::*;
use runtime::{cleanup_current_portable_runtime_dir, prepare_portable_runtime, spawn_runtime_cleanup_helper};
use service_bridge::start_service_bridge_listener;
use updater::{app_sha256, install_app_update, open_external_url};
use file_picker::{read_binary_file, reveal_path_in_explorer, select_bin_file};
use thumbnails::{load_thumbnail_cache, save_thumbnail_cache};
use roms::{import_rom_files, import_rom_files_auto, import_rom_paths_auto};
use bios::{
    check_coleco_bios, check_msx_bios, save_coleco_bios_files, save_coleco_bios_paths,
    save_msx_bios_files, save_msx_bios_paths,
};
use stock::import_stock_backup;
use backups::{
    lookup_device_backups, lookup_restore_backups, lookup_stock_backups,
};
use device::read_device_info;
use backup_read::{
    read_bank2_backup, read_mcu_backup,
    read_spi_backup,
};
use flash::{
    write_bank1_firmware, write_bank2_firmware, write_spi_firmware,
};
use app_status::{app_version, runtime_status};
use bundle_lookup::latest_firmware_bundle;
use build_metrics::compute_build_metrics;
use firmware_build::build_firmware_bundle;
use startup::{show_startup_error, PortableRuntimeReadyEvent};

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
            app_version,
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
            write_spi_firmware
        ])
        .run(tauri::generate_context!());

    cleanup_current_portable_runtime_dir();

    run_result.expect("error while running tauri application");
}
