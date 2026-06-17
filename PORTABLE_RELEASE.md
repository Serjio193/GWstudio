# Portable Windows Release

Target: clean Windows 10/11 x64.

## Embedded Runtime Layout

GW Studio is started from a single executable. Build/flash tools are embedded into the executable at compile time from release-only ZIP archives.

Before building the release, create these local files:

```text
gw-studio-tauri\src-tauri\portable\tools.zip
gw-studio-tauri\src-tauri\portable\sources.zip
```

These ZIP files are ignored by git and are only used by `build.rs`.

Before creating a final `tools.zip`, complete the bundled tool license checklist in:

```text
THIRD_PARTY_TOOLS.md
third_party_licenses\
```

Helper script:

```powershell
.\scripts\create-portable-archives.ps1 `
  -ToolsRoot C:\path\to\prepared-tools `
  -SourcesRoot C:\path\to\prepared-sources
```

`tools.zip` is extracted at runtime next to the executable:

```text
GWStudioRuntime\<pid>-<timestamp>\tools\
```

Expected `tools.zip` content:

```text
gnwmanager\gnwmanager.exe
git\bin\bash.exe
git\bin\sh.exe
make\bin\mingw32-make.exe
gcc-arm-none-eabi\bin\arm-none-eabi-gcc.exe
python\python.exe
```

The exact source package folder names do not have to match this layout if the required files exist somewhere under `tools\`. GW Studio also searches recursively for:

- `make.exe` or `mingw32-make.exe`
- `arm-none-eabi-gcc.exe`
- a directory containing both `bash.exe` and `sh.exe`

`STM32CubeProgrammer`, OpenOCD, and the full `STM32CubeIDE` GUI are not required. If local firmware build remains enabled, only the command-line ARM GCC toolchain and Make are required.

Do not include `GWUnlock\upstream\payload`, `GWUnlock\upstream\prebuilt`, or existing backup/bin files in GW Studio release archives.

`sources.zip` is extracted at runtime next to the executable:

```text
GWStudioRuntime\<pid>-<timestamp>\sources\
```

Expected `sources.zip` content:

```text
game-and-watch-retro-go-sylverb\
game-and-watch-patch\
```

If `game-and-watch-patch\` is embedded in `sources.zip`, keep its upstream `COPYING` file in that source tree. GW Studio also keeps a release attribution copy under:

```text
third_party_licenses\game-and-watch-patch\
```

The temporary folder is deleted when the Tauri event loop exits.

The executable path must not contain Cyrillic characters. If the app is started from a Cyrillic path, GW Studio shows a startup error and exits. Use a Latin-only path such as:

```text
C:\GWStudio\GW Studio.exe
```

## What Must Not Be Bundled

- Game ROMs
- BIOS dumps
- Nintendo stock firmware
- User backups
- Generated `coleco_bios.h`
- Device flash dumps

## User-Provided Files

These are stored on the user's PC after manual import:

- `StockFirmware\...` for original firmware files selected by the user.
- `msx_bios\...` for MSX BIOS files.
- `coleco_bios\coleco.rom` for ColecoVision BIOS after size and SHA-1 validation.

## Clean Windows Test

1. Start from a clean Windows 10 x64 VM and Windows 11 x64 VM.
2. Do not install Python, Git, STM32CubeIDE, STM32CubeProgrammer, Make, or ARM GCC globally.
3. Install or verify Microsoft Edge WebView2 Runtime.
4. Copy only `GW Studio.exe` to a Latin-only path, for example `C:\GWStudio\`.
5. Launch `GW Studio.exe`.
6. Run `Read Device Info`.
7. Run `Backup`.
8. Build firmware with simple ROM set.
9. Build firmware with a ColecoVision ROM and verify the app asks for `coleco.rom`.
10. Drop invalid `coleco.rom` and verify hash/size rejection.
11. Drop valid `coleco.rom` and verify build resumes.
12. Flash built firmware.

## Current Code Rules

- No hardcoded developer machine paths are allowed.
- Tools must be embedded from `portable\tools.zip` for release builds.
- Sources must be embedded from `portable\sources.zip` for release builds.
- In dev builds without embedded ZIP files, tools/sources may still be read from `GameWatchBuilderData`.
- Missing tools should produce a clear error instead of silently using a developer-installed copy.
