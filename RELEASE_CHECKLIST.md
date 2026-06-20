# Release Checklist

Use this checklist before publishing a public GW Studio release.

## Build

- [ ] Confirm the version in `gw-studio-tauri/src-tauri/Cargo.toml`.
- [ ] Run frontend build:
  ```powershell
  cd .\gw-studio-tauri
  npm run build
  ```
- [ ] Run Rust checks:
  ```powershell
  cd .\gw-studio-tauri\src-tauri
  cargo check --all-targets
  ```
- [ ] Run release build:
  ```powershell
  cd .\gw-studio-tauri
  npm run tauri:build
  ```

## Safety Gate

- [ ] Run the release safety checker:
  ```powershell
  .\scripts\check-release-safety.ps1
  ```
- [ ] Confirm the checker does not report:
  - stock Nintendo firmware;
  - BIOS files;
  - commercial ROMs;
  - user backups;
  - generated Bank1/Bank2/SPI firmware;
  - build workspaces;
  - private keys;
  - `.sig` files committed into git.

## Signing

- [ ] Generate SHA256 for the release executable:
  ```powershell
  $builtExe = ".\gw-studio-tauri\src-tauri\target\release\gw_studio_tauri.exe"
  $exe = ".\gw-studio-tauri\src-tauri\target\release\GWStudio.exe"
  Copy-Item -LiteralPath $builtExe -Destination $exe -Force
  $sha = (Get-FileHash -Algorithm SHA256 -LiteralPath $exe).Hash.ToLowerInvariant()
  Set-Content -LiteralPath "$exe.sha256" -Value "$sha  GWStudio.exe" -Encoding ASCII
  ```
- [ ] Sign the executable with the local release key:
  ```powershell
  ssh-keygen -Y sign -f .\secrets\gwstudio_release_ed25519_v2 -n gwstudio-release .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe
  ```
- [ ] Verify the signature:
  ```powershell
  cmd /c "type ""%CD%\gw-studio-tauri\src-tauri\target\release\GWStudio.exe"" | ssh-keygen -Y verify -f ""%CD%\release_keys\allowed_signers"" -I gwstudio-release -n gwstudio-release -s ""%CD%\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sig"""
  ```

## Release Upload

Upload exactly these runtime artifacts:

- [ ] `GWStudio.exe`
- [ ] `GWStudio.exe.sha256`
- [ ] `GWStudio.exe.sig`
- [ ] Confirm these exact asset names are used. The updater rejects fallback `.exe` / `.sig` names.

Do not upload:

- [ ] `secrets/`
- [ ] `GameWatchBuilderData/`
- [ ] `GWStudioRuntime/`
- [ ] `StockFirmware/`
- [ ] user `backups/`
- [ ] generated build workspaces
- [ ] stock firmware
- [ ] BIOS
- [ ] ROMs
- [ ] user saves

## Smoke Test

- [ ] Start the portable exe from a clean folder.
- [ ] Confirm startup SHA is shown.
- [ ] Read console info.
- [ ] Build firmware with a small safe ROM set.
- [ ] Confirm Auto Flash is disabled before build and enabled after a current build.
- [ ] Confirm update check does not install without `.sha256` and `.sig`.
- [ ] Confirm updater leaves `GWStudio.exe.rollback` after a successful update.
- [ ] Confirm `README.md`, `THIRD_PARTY_NOTICES.md`, and `RELEASE_AUDIT.md` match the release contents.
