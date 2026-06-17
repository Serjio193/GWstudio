# Third-Party Tools Audit

This file records the exact bundled tool categories needed by the portable release.

The final `tools.zip` is not committed to git. Before building a release, verify every bundled binary and keep its license notice in both:

- the upstream tool folder inside `tools.zip`, when the upstream package includes license files;
- `third_party_licenses/<tool>/`, for release attribution.

## Required By Current Code

### gnwmanager

Used for:

- device info
- SPI backup
- SPI flash
- fallback flows where the Python helpers call gnwmanager APIs

Expected file:

```text
tools\gnwmanager\gnwmanager.exe
```

License action:

- add upstream license file to `third_party_licenses/gnwmanager/`
- keep license files bundled with `gnwmanager.exe`, if present

### Python Runtime

Used when `gnwmanager.exe` is not sufficient and for helper scripts:

- `pyocd`
- inline CPU/voltage diagnostics
- SPI progress helper
- Retro-Go `parse_roms.py`
- game-and-watch-patch `patch.py`

Expected file:

```text
tools\python\python.exe
```

License action:

- add Python Software Foundation license to `third_party_licenses/python/`
- keep bundled Python `LICENSE.txt`
- include licenses for installed Python packages

### pyOCD

Used for:

- UID read fallback
- CPU/voltage diagnostics
- gnwmanager pyOCD backend

Expected inside Python environment:

```text
tools\python\Lib\site-packages\pyocd\
```

License action:

- add upstream pyOCD license to `third_party_licenses/pyocd/`
- include licenses for pyOCD dependencies if redistributed

### Git for Windows / Bash

Used because Retro-Go and game-and-watch-patch build commands run through `bash.exe`.

Expected files:

```text
tools\git\bin\bash.exe
tools\git\bin\sh.exe
```

The bundled Git tree may also need `usr\bin\`, `mingw64\bin\`, DLLs, and license files.

License action:

- add Git for Windows / Git / MSYS2 license notices to `third_party_licenses/git-for-windows/`
- keep license files from the redistributed Git package

### GNU Make

Used for:

- Retro-Go build
- game-and-watch-patch build

Expected file, one of:

```text
tools\make\bin\mingw32-make.exe
tools\make\bin\make.exe
```

GW Studio also searches recursively under `tools\` for `mingw32-make.exe` or `make.exe`.

License action:

- add GNU Make license to `third_party_licenses/gnu-make/`
- keep package license files in `tools.zip`

### GNU Arm Embedded Toolchain / arm-none-eabi

Used for:

- local Retro-Go ARM firmware compilation
- `arm-none-eabi-gcc`
- `arm-none-eabi-objcopy`
- `arm-none-eabi-objdump`
- linker/runtime libraries

Expected file:

```text
tools\gcc-arm-none-eabi\bin\arm-none-eabi-gcc.exe
```

GW Studio also searches recursively under `tools\` for `arm-none-eabi-gcc.exe`.

License action:

- add toolchain license notices to `third_party_licenses/gcc-arm-none-eabi/`
- keep all license/COPYING files from the redistributed toolchain

## Required Sources

### game-and-watch-retro-go-sylverb

Used for:

- Retro-Go Bank2 firmware build
- SPI image build

Expected in `sources.zip`:

```text
sources\game-and-watch-retro-go-sylverb\
```

License action:

- keep upstream license files inside the source tree
- preserve attribution in README

### game-and-watch-patch

Used for:

- local patching of user-provided stock Bank1 into dualboot Bank1

Expected in `sources.zip`:

```text
sources\game-and-watch-patch\
```

License action:

- keep upstream `COPYING` inside the source tree
- release attribution is stored in `third_party_licenses/game-and-watch-patch/`

## Not Required

These are not called by current code and should not be bundled:

- STM32CubeProgrammer
- full STM32CubeIDE GUI
- OpenOCD
- Node.js / npm
- Visual Studio Build Tools
- WiX / NSIS installer tools

## Must Not Be Bundled

- Nintendo stock firmware
- generated dualboot Bank1
- BIOS dumps
- MSX BIOS files
- `coleco.rom`
- game ROMs
- user backups
- generated `coleco_bios.h`
