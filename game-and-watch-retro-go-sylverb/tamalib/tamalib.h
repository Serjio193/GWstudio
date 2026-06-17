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
#ifndef _TAMALIB_H_
#define _TAMALIB_H_

#include "tamalib_cpu.h"
#include "tamalib_hal.h"
#include "tamalib_hw.h"

#define tamalib_set_button(btn, state) hw_set_button(btn, state)

#define tamalib_get_state() cpu_get_state()
#define tamalib_refresh_hw() cpu_refresh_hw()

#define tamalib_reset() tama_cpu_reset()

bool_t tamalib_init(const u12_t *program, u32_t freq);

void tamalib_register_hal(hal_t *hal);

void tamalib_step(void);

void tamalib_set_silent(bool_t silent);

#endif /* _TAMALIB_H_ */
