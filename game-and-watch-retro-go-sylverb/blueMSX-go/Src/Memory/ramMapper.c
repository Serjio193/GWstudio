/*****************************************************************************
** $Source: /cygdrive/d/Private/_SVNROOT/bluemsx/blueMSX/Src/Memory/ramMapper.c,v $
**
** $Revision: 1.21 $
**
** $Date: 2009-07-03 21:27:14 $
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
#include "ramMapper.h"
#include "romMapperDRAM.h"
#include "ramMapperIo.h"
#include "MediaDb.h"
#include "SlotManager.h"
#include "DeviceManager.h"
#ifndef TARGET_GNW
#include "DebugDeviceManager.h"
#else
#include "gw_malloc.h"
#endif
#include "SaveState.h"
#include "IoPort.h"
#include "Language.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>


typedef struct {
    int deviceHandle;
    UInt8* ramData;
    int handle;
#ifndef TARGET_GNW
    int debugHandle;
#endif
    int dramHandle;
    int dramMode;
    UInt8 port[4];
    int slot;
    int sslot;
    int mask;
    int size;
} RamMapper;

#ifdef TARGET_GNW
// 128kB of RAM maximum
UInt8 msxRam_global[0x4000*8];
#endif

static void writeIo(void* rmv, UInt16 page, UInt8 value);

static void saveState(void* rmv)
{
    RamMapper* rm = (RamMapper *)rmv;
    SaveState* state = saveStateOpenForWrite("mapperRam");
    
    saveStateSet(state, "mask",     rm->mask);
    saveStateSet(state, "dramMode", rm->dramMode);

    saveStateSetBuffer(state, "port", rm->port, 4);
    saveStateSetBuffer(state, "ramData", rm->ramData, 0x4000 * (rm->mask + 1));

    saveStateClose(state);
}

static void loadState(void* rmv)
{
    RamMapper* rm = (RamMapper *)rmv;
    SaveState* state = saveStateOpenForRead("mapperRam");
    int i;
    
    rm->mask     = saveStateGet(state, "mask", 0);
    rm->dramMode = saveStateGet(state, "dramMode", 0);
    
    saveStateGetBuffer(state, "port", rm->port, 4);
    saveStateGetBuffer(state, "ramData", rm->ramData, 0x4000 * (rm->mask + 1));

    saveStateClose(state);

#if 1
    for (i = 0; i < 4; i++) {
        writeIo(rm, i, rm->port[i]);
    }
#else
    ramMapperIoRemove(rm->handle);
    rm->handle  = ramMapperIoAdd(0x4000 * (rm->mask + 1), writeIo, rm);

    for (i = 0; i < 4; i++) {
        int value = ramMapperIoGetPortValue(i) & rm->mask;
        int mapped = rm->dramMode && (rm->mask - value < 4) ? 0 : 1;
        slotMapPage(rm->slot, rm->sslot, 2 * i,     rm->ramData + 0x4000 * value, 1, mapped);
        slotMapPage(rm->slot, rm->sslot, 2 * i + 1, rm->ramData + 0x4000 * value + 0x2000, 1, mapped);
    }
#endif
}

static void writeIo(void* rmv, UInt16 page, UInt8 value)
{
    RamMapper* rm = (RamMapper *)rmv;
    int baseAddr = 0x4000 * (value & rm->mask);
    rm->port[page] = value;
    if (rm->dramMode && baseAddr >= (rm->size - 0x10000)) {
        slotMapPage(rm->slot, rm->sslot, 2 * page,     NULL, 0, 0);
        slotMapPage(rm->slot, rm->sslot, 2 * page + 1, NULL, 0, 0);
    }
    else {
        slotMapPage(rm->slot, rm->sslot, 2 * page,     rm->ramData + baseAddr, 1, 1);
        slotMapPage(rm->slot, rm->sslot, 2 * page + 1, rm->ramData + baseAddr + 0x2000, 1, 1);
    }
}

static void setDram(void* rmv, int enable)
{
    RamMapper* rm = (RamMapper *)rmv;
    int i;

    rm->dramMode = enable;

    for (i = 0; i < 4; i++) {
        writeIo(rm, i, ramMapperIoGetPortValue(i));
    }
}

static void reset(RamMapper* rm)
{
    setDram(rm, 0);
}

static void destroy(void* rmv)
{
    RamMapper* rm = (RamMapper *)rmv;
#ifndef TARGET_GNW
    debugDeviceUnregister(rm->debugHandle);
#endif
    ramMapperIoRemove(rm->handle);
    slotUnregister(rm->slot, rm->sslot, 0);
    deviceManagerUnregister(rm->deviceHandle);
    panasonicDramUnregister(rm->dramHandle);
#ifndef MSX_NO_MALLOC
    free(rm->ramData);

    free(rm);
#endif
}

#ifndef TARGET_GNW
static void getDebugInfo(RamMapper* rm, DbgDevice* dbgDevice)
{
    dbgDeviceAddMemoryBlock(dbgDevice, langDbgMemRamMapped(), 0, 0, rm->size, rm->ramData);
}

static int dbgWriteMemory(RamMapper* rm, char* name, void* data, int start, int size)
{
    if (strcmp(name, "Mapped") || start + size > rm->size) {
        return 0;
    }

    memcpy(rm->ramData + start, data, size);

    return 1;
}
#endif

int ramMapperCreate(int size, int slot, int sslot, int startPage, UInt8** ramPtr, UInt32* ramSize) 
{
    DeviceCallbacks callbacks = { destroy, NULL, saveState, loadState };
#ifndef TARGET_GNW
    DebugCallbacks dbgCallbacks = { getDebugInfo, dbgWriteMemory, NULL, NULL };
#endif
    RamMapper* rm;
    int pages = size / 0x4000;
    int i;

    // Check that memory is a power of 2 and at least 64kB
    for (i = 4; i < pages; i <<= 1);

    if (i != pages) {
        return 0;
    }

    size = pages * 0x4000;

    // Start page must be zero (only full slot allowed)
    if (startPage != 0) {
        return 0;
    }

#ifndef TARGET_GNW
    rm = malloc(sizeof(RamMapper));
    rm->ramData  = malloc(size);
#else
    rm = itc_malloc(sizeof(RamMapper));
    if (pages > 8) {
        printf("Tried to allocate more than 8 pages of RAM : %d",pages);
        return 0; // No more than 128kB of ram supported
    }
    rm->ramData  = msxRam_global;
#endif

    rm->size     = size;
    rm->slot     = slot;
    rm->sslot    = sslot;
    rm->mask     = pages - 1;
    rm->dramMode = 0;

    memset(rm->ramData, 0xff, size);

    rm->handle  = ramMapperIoAdd(pages * 0x4000, writeIo, rm);
    
#ifndef TARGET_GNW
    rm->debugHandle = debugDeviceRegister(DBGTYPE_RAM, langDbgDevRam(), &dbgCallbacks, rm);
#endif

    rm->deviceHandle = deviceManagerRegister(RAM_MAPPER, &callbacks, rm);
    slotRegister(slot, sslot, 0, 8, NULL, NULL, NULL, destroy, rm);

    reset(rm);

    if (ramPtr != NULL) {
        // Main RAM
        rm->dramHandle = panasonicDramRegister(setDram, rm);
        *ramPtr = rm->ramData;
    }
    if (ramSize != NULL) {
        *ramSize = size;
    }

    return 1;
}

#endif
