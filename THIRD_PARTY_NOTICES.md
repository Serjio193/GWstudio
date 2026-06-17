# Third-Party Notices

GW Studio release binaries may bundle third-party runtime and build components inside a portable executable. This file is a summary for release users and does not replace the full upstream license texts.

Third-party components bundled with GW Studio remain under their own respective licenses.

## Important Packaging Note

GW Studio is distributed as a portable Windows executable. On startup, embedded runtime archives are extracted next to the executable into `GWStudioRuntime\`.

The ST-Link USB driver is **not** bundled with GW Studio and must be installed separately by the user from the official vendor/source.

GW Studio does not bundle Nintendo stock firmware, commercial game ROMs, BIOS dumps, user backups, console flash dumps, or generated dualboot firmware images.

## Bundled / Embedded Components

| Component | Version | License | Project / Source |
|---|---:|---|---|
| sylverb game-and-watch-retro-go fork | source snapshot | upstream project licenses | https://github.com/sylverb/game-and-watch-retro-go |
| game-and-watch-patch | source snapshot | BSD-style license, see `licenses/game-and-watch-patch/COPYING.txt` | https://github.com/BrianPugh/game-and-watch-patch |
| Python | 3.13.2, exact bundled environment must be verified | Python Software Foundation License Version 2 | https://www.python.org/ |
| gnwmanager | 0.21.1 | Apache-2.0 | https://github.com/BrianPugh/gnwmanager |
| pyOCD | 0.44.1 | Apache-2.0 | https://github.com/pyocd/pyOCD |
| GNU Make | 4.4.1_st_20231030-1220 if reused from STM32CubeIDE command-line tools; verify exact bundled package | GPL-3.0-or-later | https://www.gnu.org/software/make/ |
| GNU Arm Embedded Toolchain / arm-none-eabi | GNU Tools for STM32 14.3.rel1.20251027-0700 / GCC 14.3.1 if reused from STM32CubeIDE command-line tools; verify exact bundled package | mixed GCC/binutils/newlib/runtime licenses | https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads |
| Git for Windows / Bash / MSYS2 runtime | Git 2.52.0.windows.1 / bash 5.2.37 if reused from local Git for Windows; verify exact bundled package | mixed Git/GPL/MSYS2/MinGW licenses | https://gitforwindows.org/ |

## External User-Provided Files

The bundled `gnwmanager` Python package keeps its RAM helper `firmware.bin`, which GW Studio needs for SPI operations. GW Studio removes `gnwmanager/unlock.bin` and `gnwmanager/cli/gnw_patch/binaries/` from the portable Python runtime because those payloads are not used by GW Studio.

Some emulator targets require user-provided files. GW Studio stores these on the user's PC after manual import and does not ship them:

- Nintendo stock firmware backups selected by the user.
- MSX BIOS files.
- ColecoVision `coleco.rom`, accepted only after size and SHA-1 validation.
- Game ROMs selected by the user.

## Firmware / ROM Disclaimer

GW Studio does not include, distribute, or provide Nintendo firmware, ROMs, BIOS files, game data, copyrighted images, or other proprietary Nintendo content.

Users are responsible for creating and preserving backups from their own devices and for complying with all applicable laws and regulations in their jurisdiction.

## Affiliation Disclaimer

This project is not affiliated with, endorsed by, or associated with Nintendo, STMicroelectronics, Tauri, or any other company or upstream project mentioned in this repository.

## License Texts

License texts and notices for bundled third-party components are included in the `licenses/` directory and should be distributed together with binary releases.

For build tools, exact license files must be verified against the exact binary packages used to create `tools.zip`.
