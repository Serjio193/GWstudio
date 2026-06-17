/*****************************************************************************
** $Source: /cygdrive/d/Private/_SVNROOT/bluemsx/blueMSX/Src/IoDevice/SunriseIDE.c,v $
**
** $Revision: 1.9 $
**
** $Date: 2008-03-30 18:38:40 $
**
** More info: http://www.bluemsx.com
**
** Copyright (C) 2003-2006 Daniel Vik
**
** This program is free software; you can redistribute it and/or modify
** it under the terms of the GNU General Public License as published by
** the Free Software Foundation; either version 2 of the License, or
** (at your option) any later version.
** 
** This program is distributed in the hope that it will be useful,
** but WITHOUT ANY WARRANTY; without even the implied warranty of
** MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
** GNU General Public License for more details.
**
** You should have received a copy of the GNU General Public License
** along with this program; if not, write to the Free Software
** Foundation, Inc., 675 Mass Ave, Cambridge, MA 02139, USA.
**
******************************************************************************
*/
#ifdef TARGET_GNW
#include "build/config.h"
#endif

#if !defined(TARGET_GNW) || (defined(TARGET_GNW) &&  defined(ENABLE_EMULATOR_MSX))
#include "SunriseIDE.h"
#include "HarddiskIDE.h"
#include "Board.h"
#include "SaveState.h"
#include "Disk.h"
#include <stdlib.h>
#include <string.h>
#ifdef TARGET_GNW
#include "gw_malloc.h"
#endif


struct SunriseIde {
    int softReset;
    int currentDevice;
#ifndef TARGET_GNW
    HarddiskIde* hdide[2];
#else
    HarddiskIde* hdide[1];
#endif
};


SunriseIde* sunriseIdeCreate(int hdId)
{
#ifndef TARGET_GNW
    SunriseIde* ide = malloc(sizeof(SunriseIde));
#else
    SunriseIde* ide = itc_malloc(sizeof(SunriseIde));
#endif

    ide->hdide[0] = harddiskIdeCreate(diskGetHdDriveId(hdId, 0));
#ifndef TARGET_GNW
    ide->hdide[1] = harddiskIdeCreate(diskGetHdDriveId(hdId, 1));
#endif

    sunriseIdeReset(ide);

    return ide;
}

void sunriseIdeDestroy(SunriseIde* ide)
{
    harddiskIdeDestroy(ide->hdide[0]);
#ifndef TARGET_GNW
    harddiskIdeDestroy(ide->hdide[1]);
    free(ide);
#endif
}

void sunriseIdeReset(SunriseIde* ide)
{
    ide->currentDevice = 0;
    ide->softReset = 0;
    harddiskIdeReset(ide->hdide[0]);
#ifndef TARGET_GNW
    harddiskIdeReset(ide->hdide[1]);
#endif
}

UInt16 sunriseIdeRead(SunriseIde* ide)
{
#ifndef TARGET_GNW
    return harddiskIdeRead(ide->hdide[ide->currentDevice]);
#else
    if (ide->currentDevice == 0) {
        return harddiskIdeRead(ide->hdide[0]);
    } else {
        return 0x7f7f;
    }
#endif
}

#ifndef TARGET_GNW
UInt16 sunriseIdePeek(SunriseIde* ide)
{
    return harddiskIdePeek(ide->hdide[ide->currentDevice]);
}
#endif

void sunriseIdeWrite(SunriseIde* ide, UInt16 value)
{
#ifndef TARGET_GNW
    harddiskIdeWrite(ide->hdide[ide->currentDevice], value);
#endif
}

UInt8 sunriseIdeReadRegister(SunriseIde* ide, UInt8 reg)
{
    UInt8 value;

    if (reg == 14) {
        reg = 7;
    }

    if (ide->softReset) {
        return 0x7f | (reg == 7 ? 0x80 : 0);
    }

    if (reg == 0) {
        return sunriseIdeRead(ide) & 0xFF;
    } 

#ifndef TARGET_GNW
    value = harddiskIdeReadRegister(ide->hdide[ide->currentDevice], reg);
#else
    if (ide->currentDevice == 0) {
        value = harddiskIdeReadRegister(ide->hdide[ide->currentDevice], reg);
    } else {
        value = 0x7f;
    }
#endif

    if (reg == 6) {
        value = (value & ~0x10) | (ide->currentDevice << 4);
    }
    return value;
}


#ifndef TARGET_GNW
UInt8 sunriseIdePeekRegister(SunriseIde* ide, UInt8 reg)
{
    UInt8 value;

    if (reg == 14) {
        reg = 7;
    }

    if (ide->softReset) {
        return 0x7f | (reg == 7 ? 0x80 : 0);
    }

    if (reg == 0) {
        return sunriseIdePeek(ide) & 0xFF;
    } 

    value = harddiskIdePeekRegister(ide->hdide[ide->currentDevice], reg);
    if (reg == 6) {
        value = (value & ~0x10) | (ide->currentDevice << 4);
    }
    return value;
}
#endif

void sunriseIdeWriteRegister(SunriseIde* ide, UInt8 reg, UInt8 value)
{
    if (ide->softReset) {
        if ((reg == 14) && !(value & 0x04)) {
            ide->softReset = 0;
        }
        return;
    }

    if (reg == 0) {
        sunriseIdeWrite(ide, (value << 8) | value);
        return;
    }

    if ((reg == 14) && (value & 0x04)) {
        ide->softReset = 1;
        harddiskIdeReset(ide->hdide[0]);
#ifndef TARGET_GNW
        harddiskIdeReset(ide->hdide[1]);
#endif
        return;
    }

    if (reg == 6) {
        ide->currentDevice = (value >> 4) & 1;
    }
#ifndef TARGET_GNW
    harddiskIdeWriteRegister(ide->hdide[ide->currentDevice], reg, value);
#else
    if (ide->currentDevice == 0) {
        harddiskIdeWriteRegister(ide->hdide[ide->currentDevice], reg, value);
    }
#endif
}

void sunriseIdeLoadState(SunriseIde* ide)
{
    SaveState* state = saveStateOpenForRead("sunriseIde");

    ide->softReset     = saveStateGet(state, "softReset",     0);
    ide->currentDevice = saveStateGet(state, "currentDevice", 0);

    saveStateClose(state);

    harddiskIdeLoadState(ide->hdide[0]);
#ifndef TARGET_GNW
    harddiskIdeLoadState(ide->hdide[1]);
#endif
}

void sunriseIdeSaveState(SunriseIde* ide)
{
    SaveState* state = saveStateOpenForWrite("sunriseIde");

    saveStateSet(state, "softReset",     ide->softReset);
    saveStateSet(state, "currentDevice", ide->currentDevice);

    saveStateClose(state);
    
    harddiskIdeSaveState(ide->hdide[0]);
#ifndef TARGET_GNW
    harddiskIdeSaveState(ide->hdide[1]);
#endif
}
#endif
