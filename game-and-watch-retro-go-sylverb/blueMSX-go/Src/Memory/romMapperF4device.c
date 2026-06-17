/*****************************************************************************
** $Source: /cygdrive/d/Private/_SVNROOT/bluemsx/blueMSX/Src/Memory/romMapperF4device.c,v $
**
** $Revision: 1.8 $
**
** $Date: 2008-03-30 18:38:43 $
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
#include "romMapperF4device.h"
#include "MediaDb.h"
#include "DeviceManager.h"
#include "DebugDeviceManager.h"
#include "SaveState.h"
#include "IoPort.h"
#include "Language.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#ifdef TARGET_GNW
#include "gw_malloc.h"
#endif

typedef struct {
    int deviceHandle;
#ifndef TARGET_GNW
    int debugHandle;
#endif
    int inverted;
    int status;
} RomMapperF4device;

static void saveState(void* rmv)
{
    RomMapperF4device *rm = (RomMapperF4device *)rmv;
    SaveState* state = saveStateOpenForWrite("mapperF4device");

    saveStateSet(state, "status", rm->status);

    saveStateClose(state);
}

static void loadState(void* rmv)
{
    RomMapperF4device *rm = (RomMapperF4device *)rmv;
    SaveState* state = saveStateOpenForRead("mapperF4device");
    
    rm->status = saveStateGet(state, "status", 0);

    saveStateClose(state);
}

static void destroy(void* rmv)
{
    RomMapperF4device *rm = (RomMapperF4device *)rmv;
    deviceManagerUnregister(rm->deviceHandle);
#ifndef TARGET_GNW
    debugDeviceUnregister(rm->debugHandle);
#endif

    ioPortUnregister(0xf4);

#ifndef MSX_NO_MALLOC
    free(rm);
#endif
}

static UInt8 read(void* rm, UInt16 ioPort)
{
    return ((RomMapperF4device *)rm)->status;
}

static void write(void* rmv, UInt16 ioPort, UInt8 value)
{	
    RomMapperF4device *rm = (RomMapperF4device *)rmv;
	if (rm->inverted) {
		rm->status = value | 0x7f;
	} 
    else {
		rm->status = (rm->status & 0x20) | (value & 0xa0);
	}
}

static void reset(void* rm)
{
    ((RomMapperF4device *)rm)->status = ((RomMapperF4device *)rm)->inverted ? 0xff : 0;
}

#ifndef TARGET_GNW
static void getDebugInfo(RomMapperF4device* rm, DbgDevice* dbgDevice)
{
    DbgIoPorts* ioPorts;

    ioPorts = dbgDeviceAddIoPorts(dbgDevice, langDbgDevF4Device(), 1);
    dbgIoPortsAddPort(ioPorts, 0, 0xf4, DBG_IO_READWRITE, read(rm, 0xe4));
}
#endif

int romMapperF4deviceCreate(int inverted) 
{
    DeviceCallbacks callbacks = { destroy, reset, saveState, loadState };
#ifndef TARGET_GNW
    DebugCallbacks dbgCallbacks = { getDebugInfo, NULL, NULL, NULL };

    RomMapperF4device* rm = malloc(sizeof(RomMapperF4device));
#else
    RomMapperF4device* rm = itc_malloc(sizeof(RomMapperF4device));
#endif

    rm->inverted   = inverted;
    rm->deviceHandle = deviceManagerRegister(inverted ? ROM_F4INVERTED : ROM_F4DEVICE, &callbacks, rm);
#ifndef TARGET_GNW
    rm->debugHandle = debugDeviceRegister(DBGTYPE_BIOS, langDbgDevF4Device(), &dbgCallbacks, rm);
#endif

    ioPortRegister(0xf4, read, write, rm);

    reset(rm);

    return 1;
}
#endif
