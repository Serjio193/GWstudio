# GW Studio Tauri Shell

This directory is the new desktop app target:

- `React UI`
- `Tauri shell`
- `Rust backend commands`

The old PySide screen is no longer the target architecture.

## Current state

- The provided UI mockup has been ported into the React app structure.
- The Rust backend exposes initial commands:
  - `runtime_status`
  - `read_device_info`
  - `builder_placeholder`
- The frontend already falls back to mock values when Tauri/Rust is not available.

## Build requirements

This machine currently has:

- `node`
- `npm`

But it does **not** have:

- `cargo`
- `rustc`

So desktop Tauri builds will not work until Rust is installed.

## Next commands after Rust install

```powershell
cd 'E:\Game Watch\stm32_unified_fw_project\gw-studio-tauri'
npm install
npm run tauri:dev
```

## Frontend-only check

```powershell
cd 'E:\Game Watch\stm32_unified_fw_project\gw-studio-tauri'
npm install
npm run build
```
