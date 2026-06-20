# GW Studio

GW Studio is a portable Windows application for Nintendo Game & Watch modding workflows.

The program helps read console information, create backups, build a Retro-Go firmware bundle, flash the console, and restore firmware from user-owned backups or user-provided stock firmware.

## What It Does

- Reads Game & Watch hardware information through ST-LINK.
- Saves console backups under the device UID.
- Builds a dual-boot setup with stock Bank1, Retro-Go fork Bank2, and SPI image.
- Imports supported ROMs for Retro-Go cores.
- Downloads optional game menu images from public thumbnail sources.
- Uses small 96x96 firmware images for the console while keeping UI previews separate.
- Flashes Bank1, Bank2, and SPI with progress indication.
- Restores the console from UID-based backups.
- Restores original stock firmware after the user provides matching stock files.
- Checks GitHub Releases for application updates.

## What Is Not Included

GW Studio does not include Nintendo firmware, commercial games, BIOS files, or copyrighted ROMs.

You must provide your own legally obtained:

- Game & Watch stock firmware backups.
- Game ROMs.
- BIOS files required by specific emulator cores, such as ColecoVision or MSX.

## Supported System

- Windows 10 or Windows 11 x64.
- ST-LINK compatible programmer.
- Nintendo Game & Watch Mario or Zelda hardware.
- Folder path without Cyrillic characters.

The portable release is a single executable. On startup it extracts its bundled tools to a temporary runtime folder next to the exe and removes that runtime after closing.

## Download And Verify

Download the latest `GWStudio.exe` from GitHub Releases:

```text
https://github.com/Serjio193/GWstudio/releases/latest
```

Each release also provides:

- `GWStudio.exe.sha256`
- `GWStudio.exe.sig`

PowerShell verification:

```powershell
Get-FileHash -Algorithm SHA256 .\GWStudio.exe
```

The printed hash must match the first value inside `GWStudio.exe.sha256`.

Example `.sha256` format:

```text
87664067AB929B6C55B53886B9D0D71887A27BFD09C1A2A85FF8DF8A64FA2B9D  GWStudio.exe
```

## Windows SmartScreen

GW Studio is currently not code-signed. Windows may show a warning such as "unknown publisher" or "Windows protected your PC".

This warning appears because the executable is unsigned and new, not because the SHA256 check failed. To reduce risk:

- Download only from the official GitHub Releases page.
- Verify `GWStudio.exe` with the matching `.sha256` file.
- Keep the exe in a Latin-only folder path, for example `C:\GWStudio\GWStudio.exe`.

## Basic Workflow

1. Connect ST-LINK to the console.
2. Start GW Studio.
3. Press `Read Device Info`.
4. Create a backup and keep it safe.
5. Add ROMs.
6. Optional: press `Download Images` before building if you want images in the Retro-Go menu.
7. Press `Build Firmware`.
8. Press `Flash Build` to write Bank1 + Bank2 + SPI.
9. From stock firmware, press `LEFT + GAME` to start Retro-Go.

If stock firmware files are required and missing, GW Studio asks you to drop the matching original files. The program checks that Mario/Zelda stock files match the selected hardware before saving them.

## Updates

GW Studio checks the latest GitHub Release after startup. If a newer version is found, it asks before updating.

The update process:

1. Downloads the new exe from GitHub Releases.
2. Requires the official `GWStudio.exe`, `GWStudio.exe.sha256`, and `GWStudio.exe.sig` release assets.
3. Verifies SHA256 before installing.
4. Verifies the Ed25519/OpenSSH signature with the public key built into the app.
5. Closes the current application.
6. Saves `GWStudio.exe.rollback` next to the current exe.
7. Replaces the old exe.
8. Starts the new version.

You can also run the same check from Settings with `Update program`.

## Links

- GitHub Repository: https://github.com/Serjio193/GWstudio
- Say Thanks: https://www.paypal.com/paypalme/SerhiiTarnopovych

## Third-Party Components

GW Studio uses open-source third-party tools and firmware projects. See:

- `THIRD_PARTY_NOTICES.md`
- `THIRD_PARTY_TOOLS.md`
- `LICENSE_MANIFEST.md`
- `third_party_licenses/`

Retro-Go fork source:

- https://github.com/sylverb/game-and-watch-retro-go

Game & Watch patch source:

- https://github.com/BrianPugh/game-and-watch-patch
