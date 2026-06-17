Git for Windows / Bash / MSYS2 runtime

Known local package checked during release preparation:

- Git for Windows 2.52.0.windows.1
- GNU bash 5.2.37

Included notices:

- LICENSE.txt
- COPYING3-gcc-libs.txt
- COPYING.RUNTIME-gcc-libs.txt
- COPYING-libwinpthread.txt

GW Studio needs bash/sh only because the current Retro-Go and game-and-watch-patch build flow runs through shell scripts.

Before publishing a final release, verify these notices against the exact Git for Windows / MSYS2 runtime package included in tools.zip. Git for Windows ships many libraries; include additional license files if the bundled subset contains more runtime DLLs/tools.
