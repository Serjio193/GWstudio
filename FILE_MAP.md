# GWstudio test2 File Map

Generated on 2026-06-13 for quick project orientation.

## Scope

This map focuses on the useful source/layout files in:

- `E:\Game Watch\GWstudio test2`
- `E:\Game Watch\GWstudio test2\gw-studio-tauri`

Generated output folders are listed separately so they can be ignored during normal editing.

## Top Level

```text
GWstudio test2/
  CHAT_CONTEXT.md
  README.md
  start_gw_studio_test.cmd
  start_gw_studio_test.ps1
  game-and-watch-retro-go-sylverb/
  gw-studio-tauri/
```

## Main App

`gw-studio-tauri/` is the active desktop app workspace.

`game-and-watch-retro-go-sylverb/` is the local Retro-Go fork dependency used by the firmware bundle builder.

```text
gw-studio-tauri/
  index.html
  package.json
  package-lock.json
  postcss.config.js
  tailwind.config.js
  vite.config.js
  README.md
  public/
  src/
  src-tauri/
  internal_flash_backup_test.bin
  test_spi_1mb.bin
```

## Frontend

```text
src/
  main.jsx
  App.jsx
  styles.css
  lib/
    mock.js
    tauri.js
```

### Frontend roles

- `src/main.jsx`: React bootstrap and app mount.
- `src/App.jsx`: main UI, state management, panels, device/build workflow.
- `src/styles.css`: global styles and Tailwind/custom rules.
- `src/lib/tauri.js`: thin wrapper around Tauri `invoke`, window access, and event listeners.
- `src/lib/mock.js`: mock constants and fallback device data.

## Static Assets

```text
public/
  assets/
    m-console.png
    z-console.png
  emulators/
    a7800.png
    gb.png
    gbc.png
    gg.png
    gw.png
    md.png
    msx.png
    nes.png
    pce.png
    sg.png
    sms.png
    tama.png
    wsv.png
```

### Asset roles

- `public/assets/*`: console mockup images used by the UI.
- `public/emulators/*`: emulator icons shown in the builder.

## Tauri / Rust Backend

```text
src-tauri/
  Cargo.toml
  Cargo.lock
  build.rs
  tauri.conf.json
  tauri.dev.conf.json
  capabilities/
    default.json
  icons/
    icon.ico
  gen/
    schemas/
      acl-manifests.json
      capabilities.json
      desktop-schema.json
      windows-schema.json
  src/
    main.rs
    lib.rs
```

### Backend roles

- `src-tauri/src/main.rs`: Tauri runtime entrypoint.
- `src-tauri/src/lib.rs`: main Rust backend logic and commands.
- `src-tauri/build.rs`: build-time Tauri setup.
- `src-tauri/tauri.conf.json`: production Tauri config.
- `src-tauri/tauri.dev.conf.json`: development config.
- `src-tauri/capabilities/default.json`: Tauri capabilities policy.
- `src-tauri/gen/schemas/*`: generated schema files, usually not edited manually.

## Important Notes From Review

- The frontend is a single-page React shell built with Vite + Tailwind.
- The Rust side contains the real device/backup/flash command logic.
- The app appears centered on a Game & Watch builder workflow with backup, flash, bundle, and device-status actions.
- `src/App.jsx` and `src-tauri/src/lib.rs` are the two highest-value files for future work.

## Generated / Non-Source Output

These folders are generated and should usually be ignored unless debugging builds:

```text
gw-studio-tauri/
  .devlogs/
  dist/
  node_modules/
  src-tauri/target/
```

## Quick Navigation Targets

- UI entry: `gw-studio-tauri/src/App.jsx`
- React bootstrap: `gw-studio-tauri/src/main.jsx`
- Tauri backend: `gw-studio-tauri/src-tauri/src/lib.rs`
- Tauri config: `gw-studio-tauri/src-tauri/tauri.conf.json`
- Package scripts: `gw-studio-tauri/package.json`
