# Fork of ducalex/retro-go targeting the Nintendo Game and Watch stm32-based systems

This is a fork of [ducalex/retro-go](https://github.com/ducalex/retro-go) with changes made to support [kbeckmann/game-and-watch-retro-go](https://github.com/kbeckmann/game-and-watch-retro-go).

Game covers have been removed from the repository to avoid copyright issues.

# Acknowledgements
- The NES/GBC/SMS emulators and base library were originally from the "Triforce" fork of the [official Go-Play firmware](https://github.com/othercrashoverride/go-play) by crashoverride, Nemo1984, and many others.
- The [HuExpress](https://github.com/kallisti5/huexpress) (PCE) emulator was first ported to the GO by [pelle7](https://github.com/pelle7/odroid-go-pcengine-huexpress/).
- The Lynx emulator is an adaptation of [libretro-handy](https://github.com/libretro/libretro-handy).
- The aesthetics of the launcher were inspired (copied) from [pelle7's go-emu](https://github.com/pelle7/odroid-go-emu-launcher).
- [miniz](https://github.com/richgel999/miniz) For zipped ROM and zlib API
- [luPng](https://github.com/jansol/LuPng) For basic PNG decoding


# License
This project is licensed under the GPLv2. Some components are also available under the MIT license. 
Respective copyrights apply to each component. For the rest:
```
Retro-Go: Retro emulation for the ODROID-GO
Copyright (C) 2020 Alex Duchesne

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation; either version 2 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License along
with this program; if not, write to the Free Software Foundation, Inc.,
51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
```