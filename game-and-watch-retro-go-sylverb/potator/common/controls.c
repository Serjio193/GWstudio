#ifdef TARGET_GNW
#include "build/config.h"
#endif

#if !defined(TARGET_GNW) || (defined(TARGET_GNW) &&  defined(ENABLE_EMULATOR_WSV))
#include "controls.h"

static uint8 controls_state;

void controls_reset(void)
{
    controls_state = 0;
}

uint8 controls_read(void)
{
    return controls_state ^ 0xff;
}

void controls_state_write(uint8 data)
{
    controls_state = data;
}

#endif