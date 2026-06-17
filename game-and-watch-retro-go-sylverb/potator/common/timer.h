#ifndef __TIMER_H__
#define __TIMER_H__

#include "types.h"

#include <stdio.h>

void timer_reset(void);
void timer_write(uint8 data);
void timer_exec(uint32 cycles);

int32 timer_get_cycles(void);
void timer_set_cycles(int32 cycles);
BOOL timer_get_activated(void);
void timer_set_activated(BOOL activated);

#endif
