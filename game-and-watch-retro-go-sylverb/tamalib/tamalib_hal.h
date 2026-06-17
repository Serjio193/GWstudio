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
#ifndef _HAL_H_
#define _HAL_H_

#include "tamalib_hal_types.h"

#ifndef NULL
	#define NULL 0
#endif

typedef enum {
	LOG_ERROR	= 0x1,
	LOG_INFO	= (0x1 << 1),
	LOG_MEMORY	= (0x1 << 2),
	LOG_CPU		= (0x1 << 3),
} log_level_t;

/* The Hardware Abstraction Layer
 * NOTE: This structure acts as an abstraction layer between TamaLIB and the OS/SDK.
 * All pointers MUST be implemented, but some implementations can be left empty.
 */
typedef struct {
	/* What to do if the CPU has halted
	 */
	void (*halt)(void);

	/* Log related function
	 * NOTE: Needed only if log messages are required.
	 */
	bool_t (*is_log_enabled)(log_level_t level);
	void (*log)(log_level_t level, char *buff, ...);

	/* Screen related functions */
	void (*set_lcd_matrix)(u8_t x, u8_t y, bool_t val);
	void (*set_lcd_icon)(u8_t icon, bool_t val);

	/* Sound related functions
	 * NOTE: set_frequency() changes the output frequency of the sound in dHz, while
	 * play_frequency() decides whether the sound should be heard or not.
	 */
	void (*set_sound_period)(u8_t freq);
	void (*play_sound)(bool_t en);

} hal_t;

extern hal_t *g_hal;

#endif /* _HAL_H_ */
