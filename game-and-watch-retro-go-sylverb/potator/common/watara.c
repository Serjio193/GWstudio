#ifdef TARGET_GNW
#include "build/config.h"
#endif

#if !defined(TARGET_GNW) || (defined(TARGET_GNW) &&  defined(ENABLE_EMULATOR_WSV))
#ifdef _MSC_VER
#define _CRT_SECURE_NO_WARNINGS
#endif

#include "supervision.h"

#include "controls.h"
#include "gpu.h"
#include "memorymap.h"
#include "wsv_sound.h"
#include "timer.h"
#include "./m6502/m6502.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static M6502 m6502_registers;
static BOOL irq = FALSE;

void m6502_set_irq_line(BOOL assertLine)
{
    m6502_registers.IRequest = assertLine ? INT_IRQ : INT_NONE;
    irq = assertLine;
}

byte Loop6502(register M6502 *R)
{
    if (irq) {
        irq = FALSE;
        return INT_IRQ;
    }
    return INT_QUIT;
}

void supervision_init(void)
{
    gpu_init();
    memorymap_init();
    // 256 * 256 -- 1 frame (61 FPS)
    // 256 - 4MHz,
    // 512 - 8MHz, ...
    m6502_registers.IPeriod = 256;
}

void supervision_reset(void)
{
    controls_reset();
    gpu_reset();
    memorymap_reset();
    supervision_sound_reset();
    timer_reset();

    Reset6502(&m6502_registers);
    irq = FALSE;
}

void supervision_done(void)
{
    gpu_done();
    memorymap_done();
}

BOOL supervision_load(const uint8 *rom, uint32 romSize)
{
    if (!memorymap_load(rom, romSize)) {
        return FALSE;
    }
    supervision_reset();
    return TRUE;
}

void supervision_exec(uint16 *backbuffer)
{
    supervision_exec_ex(backbuffer, SV_W);
}

void supervision_exec_ex(uint16 *backbuffer, int16 backbufferWidth)
{
    uint32 i, scan;
    uint8 *regs = memorymap_getRegisters();
    uint8 innerx, size;

    // Number of iterations = 256 * 256 / m6502_registers.IPeriod
    for (i = 0; i < 256; i++) {
        Run6502(&m6502_registers);
        timer_exec(m6502_registers.IPeriod);
    }

    //if (!(regs[BANK] & 0x8)) { printf("LCD off\n"); }
    scan   = regs[XPOS] / 4 + regs[YPOS] * 0x30;
    innerx = regs[XPOS] & 3;
    size   = regs[XSIZE]; // regs[XSIZE] <= SV_W
    if (size > SV_W)
        size = SV_W; // 192: Chimera, Matta Blatta, Tennis Pro '92

    for (i = 0; i < SV_H; i++) {
        if (scan >= 0x1fe0)
            scan -= 0x1fe0; // SSSnake
        gpu_render_scanline(scan, backbuffer, innerx, size);
        backbuffer += backbufferWidth;
        scan += 0x30;
    }

    if (Rd6502(0x2026) & 0x01)
        Int6502(&m6502_registers, INT_NMI);

    supervision_sound_decrement();
}

void supervision_set_map_func(SV_MapRGBFunc func)
{
    gpu_set_map_func(func);
}

void supervision_set_color_scheme(int8 colorScheme)
{
    gpu_set_color_scheme(colorScheme);
}

int supervision_get_color_scheme()
{
    return gpu_get_color_scheme();
}

void supervision_set_ghosting(int frameCount)
{
    gpu_set_ghosting(frameCount);
}

void supervision_set_input(uint8 data)
{
    controls_state_write(data);
}

void supervision_update_sound(uint8 *stream, uint32 len)
{
    sound_stream_update(stream, len);
}

int supervision_save_state(uint8 *dest_buffer)
{
    uint16 offset = 0;
    BOOL timer_activated = timer_get_activated();
    int32 timer_cycles = timer_get_cycles();
    int8 color_scheme = supervision_get_color_scheme();

    memcpy(dest_buffer+offset,"WSV1",4);
    offset+=4;
    memcpy(dest_buffer+offset,&color_scheme,sizeof(int8));
    offset+=sizeof(int8);
    memcpy(dest_buffer+offset,&timer_cycles,sizeof(int32));
    offset+=sizeof(int32);
    memcpy(dest_buffer+offset,&timer_activated,sizeof(BOOL));
    offset+=sizeof(BOOL);
    offset+=sound_save_buffer(dest_buffer+offset);
    memcpy(dest_buffer+offset,memorymap_getLowerRamPointer(),0x2000);
    offset+=0x2000;
    memcpy(dest_buffer+offset,memorymap_getUpperRamPointer(),0x2000);
    offset+=0x2000;
    memcpy(dest_buffer+offset,memorymap_getRegisters(),0x2000);
    offset+=0x2000;
    memcpy(dest_buffer+offset,&m6502_registers,sizeof(m6502_registers));
    offset+=sizeof(m6502_registers);
    return(offset);
}

int supervision_load_state(uint8 *src_buffer)
{
    uint16 offset = 0;
    BOOL timer_activated;
    int32 timer_cycles;
    int8 color_scheme;

    if (memcmp(src_buffer,"WSV1",4) == 0) {
        memorymap_reset();
        offset+=4;
        memcpy(&color_scheme,src_buffer+offset,sizeof(int8));
        offset+=sizeof(int8);
        memcpy(&timer_cycles,src_buffer+offset,sizeof(int32));
        timer_set_cycles(timer_cycles);
        offset+=sizeof(int32);
        memcpy(&timer_activated,src_buffer+offset,sizeof(BOOL));
        timer_set_activated(timer_activated);
        offset+=sizeof(BOOL);
        offset+=sound_load_buffer(src_buffer+offset);
        memcpy(memorymap_getLowerRamPointer(),src_buffer+offset,0x2000);
        offset+=0x2000;
        memcpy(memorymap_getUpperRamPointer(),src_buffer+offset,0x2000);
        offset+=0x2000;
        memcpy(memorymap_getRegisters(),src_buffer+offset,0x2000);
        offset+=0x2000;
        memcpy(&m6502_registers,src_buffer+offset,sizeof(m6502_registers));
        offset+=sizeof(m6502_registers);
        memorymap_update_lowerRomBank();
        supervision_set_color_scheme(color_scheme);

    }

    return(offset);
}

#endif