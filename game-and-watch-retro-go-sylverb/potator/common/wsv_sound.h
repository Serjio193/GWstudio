#ifndef __SOUND_H__
#define __SOUND_H__

#include "types.h"

#include <stdio.h>

void supervision_sound_reset(void);
/*!
 * Generate U8 (0 - 45), 2 channels.
 * \param len in bytes.
 */
void sound_stream_update(uint8 *stream, uint32 len);
void supervision_sound_decrement(void);
void sound_wave_write(int which, int offset, uint8 data);
void sound_dma_write(int offset, uint8 data);
void sound_noise_write(int offset, uint8 data);

uint16 sound_save_buffer(uint8 *buffer);
uint16 sound_load_buffer(uint8 *buffer);

#endif
