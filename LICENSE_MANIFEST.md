# License Manifest Checklist

Confirmed by current GW Studio documentation:

- [x] GW Studio does not bundle Nintendo stock firmware, commercial game ROMs, BIOS dumps, user backups, console flash dumps, or generated dualboot images.
- [x] `game-and-watch-patch` notice included: `licenses/game-and-watch-patch/COPYING.txt`.
- [x] libusb license notice included for Python/pyOCD USB dependencies: `licenses/libusb/COPYING.txt`.
- [x] hidapi license notices included for Python/pyOCD HID dependencies: `licenses/hidapi/`.
- [x] Python license notice copied from the local GWUnlock license set: `licenses/python/LICENSE.txt`.
- [x] gnwmanager license notice copied from the local GWUnlock license set: `licenses/gnwmanager/LICENSE.txt`.
- [x] pyOCD license notice copied from the local GWUnlock license set: `licenses/pyocd/LICENSE.txt`.
- [x] GNU Make license notice copied from local command-line tools: `licenses/gnu-make/COPYING.txt`.
- [x] GNU Arm Embedded Toolchain license notices copied from local command-line tools: `licenses/gcc-arm-none-eabi/`.
- [x] Git for Windows / Bash runtime notices copied from local Git for Windows: `licenses/git-for-windows/`.
- [x] GW Studio source license documented in top-level `LICENSE` and package metadata: Unlicense.
- [x] npm/frontend dependency license report generated: `licenses/npm-dependencies.md`.
- [x] Rust/Tauri dependency license report generated: `licenses/rust-dependencies.md`.
- [x] Current local Python environment freeze generated for audit input: `licenses/python-pip-freeze.txt`.
- [x] ST-Link USB driver: not bundled; user installs separately.

Repository license file layout:

- `licenses/python/LICENSE.txt`
- `licenses/gnwmanager/LICENSE.txt`
- `licenses/pyocd/LICENSE.txt`
- `licenses/libusb/COPYING.txt`
- `licenses/hidapi/LICENSE.txt`
- `licenses/hidapi/LICENSE-bsd.txt`
- `licenses/mingw-runtime/README.txt`
- `licenses/gnu-make/COPYING.txt`
- `licenses/gcc-arm-none-eabi/LICENSE.txt`
- `licenses/gcc-arm-none-eabi/LICENSE.THIRDPARTY.txt`
- `licenses/git-for-windows/LICENSE.txt`
- `licenses/game-and-watch-patch/COPYING.txt`
- `licenses/game-and-watch-patch/NOTICE.md`
- `licenses/npm-dependencies.md`
- `licenses/rust-dependencies.md`
- `licenses/python-pip-freeze.txt`

Current runtime rule:

- [x] OpenOCD is not bundled and is not used by GW Studio runtime code.

Needs verification before each public binary release:

- [ ] Verify `licenses/python-pip-freeze.txt` against the exact Python environment placed into `tools.zip`; regenerate after final staging.
- [ ] Verify GNU Make notices against the exact Make package bundled into `tools.zip`.
- [ ] Verify GNU Arm Embedded Toolchain notices against the exact ARM toolchain package bundled into `tools.zip`.
- [ ] Verify Git for Windows / Bash / MSYS2 notices against the exact runtime subset bundled into `tools.zip`.
- [ ] Check `tools.zip` for any additional `.dll`, `.pyd`, `.exe`, `.whl`, `.zip`, or runtime files not listed above.
- [ ] Confirm the release package includes `GW Studio.exe`, `THIRD_PARTY_NOTICES.md`, and the full `licenses/` directory.
