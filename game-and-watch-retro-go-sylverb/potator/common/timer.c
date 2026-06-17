#ifdef TARGET_GNW
#include "build/config.h"
#endif

#if !defined(TARGET_GNW) || (defined(TARGET_GNW) &&  defined(ENABLE_EMULATOR_WSV))
#include "timer.h"

#include "memorymap.h"

static int32 timer_cycles;
static BOOL  timer_activated;

void timer_reset(void)
{
    timer_cycles = 0;
    timer_activated = FALSE;
}

void timer_write(uint8 data)
{
    uint32 d = data ? data : 0x100; // Dancing Block. d = data; ???
    if ((memorymap_getRegisters()[BANK] >> 4) & 1) {
        timer_cycles = d * 0x4000; // Bubble World, Eagle Plan...
    }
    else {
        timer_cycles = d * 0x100;
    }
    timer_activated = TRUE;
}

void timer_exec(uint32 cycles)
{
    if (timer_activated) {
        timer_cycles -= cycles;

        if (timer_cycles <= 0) {
            timer_activated = FALSE;
            memorymap_set_timer_shot();
        }
    }
}

int32 timer_get_cycles(void)
{
    return timer_cycles;
}

void timer_set_cycles(int32 cycles)
{
    timer_cycles = cycles;
}

BOOL timer_get_activated(void)
{
    return timer_activated;
}

void timer_set_activated(BOOL activated)
{
    timer_activated = activated;
}

#endif