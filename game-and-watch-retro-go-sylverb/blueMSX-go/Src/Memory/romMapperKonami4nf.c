/*****************************************************************************
** $Source: /cygdrive/d/Private/_SVNROOT/bluemsx/blueMSX/Src/Memory/romMapperKonami4nf.c,v $
**
** $Revision: 1.6 $
**
** $Date: 2008-03-30 18:38:44 $
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
#include "romMapperKonami4nf.h"
#include "MediaDb.h"
#include "SlotManager.h"
#include "DeviceManager.h"
#include "SaveState.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#ifdef TARGET_GNW
#include "gw_malloc.h"
#endif


//  Uses Konami 4 but in different adresses :
//  Know Games : Animal Wars 2 , Ashguine 3 and Ashguine T&E
 

typedef struct {
    int deviceHandle;
    UInt8* romData;
    int slot;
    int sslot;
    int startPage;
    int size;
    int romMapper[4];
} RomMapperKonami4nf;

static void saveState(void* rmv)
{
    RomMapperKonami4nf *rm = (RomMapperKonami4nf *)rmv;
    SaveState* state = saveStateOpenForWrite("mapperKonami4nf");
    char tag[16];
    int i;

    for (i = 0; i < 4; i++) {
        sprintf(tag, "romMapper%d", i);
        saveStateSet(state, tag, rm->romMapper[i]);
    }

    saveStateClose(state);
}

static void loadState(void* rmv)
{
    RomMapperKonami4nf *rm = (RomMapperKonami4nf *)rmv;
    SaveState* state = saveStateOpenForRead("mapperKonami4nf");
    char tag[16];
    int i;

    for (i = 0; i < 4; i++) {
        sprintf(tag, "romMapper%d", i);
        rm->romMapper[i] = saveStateGet(state, tag, 0);
    }

    saveStateClose(state);

    for (i = 0; i < 4; i++) {   
        slotMapPage(rm->slot, rm->sslot, rm->startPage + i, rm->romData + rm->romMapper[i] * 0x2000, 1, 0);
    }
}

static void destroy(void* rmv)
{
    RomMapperKonami4nf *rm = (RomMapperKonami4nf *)rmv;
    slotUnregister(rm->slot, rm->sslot, rm->startPage);
    deviceManagerUnregister(rm->deviceHandle);

#ifdef MSX_NO_MALLOC
    free(rm->romData);
    free(rm);
#endif
}

static void write(void* rmv, UInt16 address, UInt8 value) 
{
    RomMapperKonami4nf *rm = (RomMapperKonami4nf *)rmv;
    int bank;

    address += 0x4000;

    bank = (address - 0x4000) >> 13;

    value %= rm->size / 0x2000;
    if (rm->romMapper[bank] != value) {
        UInt8* bankData = rm->romData + ((int)value << 13);

        rm->romMapper[bank] = value;
        
        slotMapPage(rm->slot, rm->sslot, rm->startPage + bank, bankData, 1, 0);
    }
}

int romMapperKonami4nfCreate(const char* filename, UInt8* romData, 
                             int size, int slot, int sslot, int startPage) 
{
    DeviceCallbacks callbacks = { destroy, NULL, saveState, loadState };
    RomMapperKonami4nf* rm;
    int i;

    if (size < 0x8000) {
        return 0;
    }

#ifndef TARGET_GNW
    rm = malloc(sizeof(RomMapperKonami4nf));
#else
    rm = itc_malloc(sizeof(RomMapperKonami4nf));
#endif

    rm->deviceHandle = deviceManagerRegister(ROM_KONAMI4NF, &callbacks, rm);
    slotRegister(slot, sslot, startPage, 4, NULL, NULL, write, destroy, rm);

#ifndef MSX_NO_MALLOC
    rm->romData = malloc(size);
    memcpy(rm->romData, romData, size);
#else
    rm->romData = romData;
#endif
    rm->size = size;
    rm->slot  = slot;
    rm->sslot = sslot;
    rm->startPage  = startPage;

    rm->romMapper[0] = 0;
    rm->romMapper[1] = 1;
    rm->romMapper[2] = 2;
    rm->romMapper[3] = 3;

    for (i = 0; i < 4; i++) {   
        slotMapPage(rm->slot, rm->sslot, rm->startPage + i, rm->romData + rm->romMapper[i] * 0x2000, 1, 0);
    }

    return 1;
}

#endif
