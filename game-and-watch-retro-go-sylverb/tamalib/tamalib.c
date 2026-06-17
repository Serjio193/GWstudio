/*
 * TamaLIB - A hardware agnostic Tamagotchi P1 emulation library
 *
 * Copyright (C) 2021 Jean-Christophe Rona <jc@rona.fr>
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.
 */
#include "tamalib.h"

hal_t *g_hal;

bool_t tamalib_init(const u12_t *program, u32_t freq) {
    bool_t res = 0;

    res |= tama_cpu_init(program, freq);
    res |= hw_init();
    cpu_set_silent(0);

    return res;
}

void tamalib_register_hal(hal_t *hal) {
    g_hal = hal;
}

void inline tamalib_step(void) {
    cpu_step();
}

void inline tamalib_set_silent(bool_t silent) {
    cpu_set_silent(silent);
}
