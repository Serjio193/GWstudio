# Release Audit

This audit records files removed or reviewed before publishing GW Studio.

## Retro-Go Fork Source

GW Studio includes the fork:

- `game-and-watch-retro-go-sylverb/`
- upstream remote in the source copy: `https://github.com/sylverb/game-and-watch-retro-go.git`

The fork contains several submodules. The files below came from those submodules, not from GW Studio application code.

## Removed From Release Copy

Removed binary BIOS/ROM files:

- `game-and-watch-retro-go-sylverb/blueMSX-go/system/bluemsx/Machines/**/*.rom`

Origin:

- submodule: `blueMSX-go`
- submodule URL in `.gitmodules`: `https://github.com/sylverb/blueMSX-go.git`

Reason:

- BIOS/ROM binaries must not be published with GW Studio.

Removed Zelda3 reference save files:

- `game-and-watch-retro-go-sylverb/zelda3/saves/ref/*.sav`

Origin:

- submodule: `zelda3`
- submodule URL in `.gitmodules`: `https://github.com/sylverb/zelda3.git`
- local source-copy git history showed these under commit `02add69 Initial version`

Reason:

- Save-state/reference files are not needed for GW Studio release and may contain game-derived data.

Removed embedded BIOS/ROM headers:

- `game-and-watch-retro-go-sylverb/retro-go-stm32/smsplusgx-go/components/smsplus/coleco_bios.h`
- `game-and-watch-retro-go-sylverb/caprice32-go/cap32/rom/464.h`
- `game-and-watch-retro-go-sylverb/caprice32-go/cap32/rom/6128.h`
- `game-and-watch-retro-go-sylverb/caprice32-go/cap32/rom/6128p.h`
- `game-and-watch-retro-go-sylverb/caprice32-go/cap32/rom/amsdos.h`
- `game-and-watch-retro-go-sylverb/caprice32-go/cap32/rom/cpm.h`

Origins:

- `coleco_bios.h`: submodule `retro-go-stm32`, URL `https://github.com/sylverb/retro-go-stm32.git`, local source-copy history showed commit `0b55bd3 coleco: Store BIOS in extflash only (no copy to RAM)`
- Amstrad ROM headers: submodule `caprice32-go`, URL `https://github.com/sylverb/caprice32-go.git`, local source-copy history showed commit `94fcecc [cpm] boot rom base`

Reason:

- These are BIOS/ROM byte arrays stored as C/C header source.

## Current Binary/ROM Audit Result

After cleanup, the release copy no longer contains obvious commercial ROM/BIOS/save/firmware extensions.

Remaining matched files:

- `game-and-watch-retro-go-sylverb/linux/Makefile.gb` - makefile, not a Game Boy ROM.
- `game-and-watch-retro-go-sylverb/linux/Makefile.nes` - makefile, not a NES ROM.

No Nintendo stock firmware, Bank1/Bank2/SPI dumps, user backups, `.sfc`, `.nes`, `.gb`, `.gbc`, or `.gba` game ROMs were found in the release copy.

## External BIOS Emulator Targets

ColecoVision (`COL`) remains enabled, but GW Studio does not ship the ColecoVision BIOS.

Release behavior:

- User must provide `coleco.rom` manually.
- GW Studio verifies size `8192` bytes and SHA-1 `2f625916c6458379379e61c91ecab3439624d8bf`.
- The verified BIOS is stored locally in app data under `coleco_bios/coleco.rom`.
- `coleco_bios.h` is generated only inside the temporary Retro-Go build workspace when a build contains `COL` games.
- No ColecoVision BIOS byte array is stored in the repository.

MSX remains enabled because GW Studio already requires users to provide MSX BIOS files through the explicit BIOS drop flow.

## Disabled/Removed Emulator Targets

Amstrad CPC (`CPC`) was removed from the GW Studio release project because its bundled upstream support depended on removed ROM data.

Removed/disabled items:

- `CPC` entry in the GW Studio emulator UI.
- `CPC` file-extension mapping in the Tauri backend.
- `CPC` ROM parser generation in `parse_roms.py`.
- `CPC` generated system from `Core/Src/retro-go/rom_manager.c`.
- Amstrad build source/object paths from Retro-Go Makefiles.
- `caprice32-go/` and `Core/*/porting/amstrad/`.
- `roms/amstrad/` and its UI icon.
- Stale `gw-studio-tauri/public/emulators.zip`, because it could contain removed emulator icons.

SNES direct-loader ports (`SMW`, `Zelda3`) were removed from the GW Studio release project because GW Studio no longer supports the direct builders and does not need to ship their reverse-engineered source trees.

Removed/disabled items:

- `smw/` and `zelda3/` source trees from the Retro-Go fork copy.
- SMW/Zelda3 generated ROM systems from `parse_roms.py` and `Core/Src/retro-go/rom_manager.c`.
- SMW/Zelda3 emulator registration and launch branches from `Core/Src/retro-go/rg_emulators.c`.
- SMW/Zelda3 linker overlays and Makefile object/build rules.
- SMW/Zelda3 porting headers/sources, redefines, asset update scripts, ROM folders, and UI icons.
- SFC import remains blocked by the GW Studio backend with a clear skipped-message.

## Remaining Release Risks

### gnwmanager Python Runtime

The portable Python runtime keeps:

- `gnwmanager/firmware.bin`

Reason:

- GW Studio calls `GnW.start_gnwmanager()` for SPI read/write/erase helpers, and `gnwmanager` loads this RAM helper firmware from `firmware.bin`.

The portable Python runtime removes:

- `gnwmanager/unlock.bin`
- `gnwmanager/cli/gnw_patch/binaries/`

Reason:

- GW Studio does not use gnwmanager's unlock flow or built-in stock patch payloads.
- Dualboot patching is handled separately through `game-and-watch-patch` and user-provided stock firmware files.

### UI Assets

The following UI preview images contain Nintendo/Game & Watch/Mario/Zelda branding:

- `gw-studio-tauri/public/assets/m-console.png`
- `gw-studio-tauri/public/assets/z-console.png`

These are not firmware/BIOS/ROM files, but they are still trademark/copyright-sensitive for a public GitHub release. Replace them with neutral mockups before publishing.

## Release Rule

Do not publish:

- commercial game ROMs
- BIOS dumps
- Nintendo stock firmware
- user backups
- console flash dumps
- generated build workspaces
- branded UI mockups unless we intentionally accept trademark risk
