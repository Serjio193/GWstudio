/*****************************************************************************
** $Source: /cygdrive/d/Private/_SVNROOT/bluemsx/blueMSX/Src/IoDevice/MsxPPI.c,v $
**
** $Revision: 1.20 $
**
** $Date: 2008-09-09 04:40:32 $
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
#include "MsxPPI.h"
#include "MediaDb.h"
#include "DeviceManager.h"
#ifndef TARGET_GNW
#include "DebugDeviceManager.h"
#endif
#include "SlotManager.h"
#include "IoPort.h"
#include "I8255.h"
#include "Board.h"
#include "SaveState.h"
#ifndef TARGET_GNW
#include "KeyClick.h"
#endif
#include "ArchInput.h"
#include "Switches.h"
#include "Led.h"
#include "InputEvent.h"
#include "Language.h"
#include "Properties.h"
#ifndef TARGET_GNW
#include "DAC.h"
#endif
#include <stdlib.h>
#ifdef TARGET_GNW
#include "gw_malloc.h"
#endif


static UInt8 getKeyState(int row);


typedef struct {
    int    deviceHandle;
#ifndef TARGET_GNW
    int    debugHandle;
#endif
    I8255* i8255;

#ifndef TARGET_GNW
    AudioKeyClick* keyClick;
    DAC*   dac;
#endif

    UInt8 row;
    Int32 regA;
    Int32 regCHi;
} MsxPPI;

static void destroy(void* ppiv)
{
    MsxPPI *ppi = (MsxPPI *)ppiv;
    ioPortUnregister(0xa8);
    ioPortUnregister(0xa9);
    ioPortUnregister(0xaa);
    ioPortUnregister(0xab);

#ifndef TARGET_GNW
    audioKeyClickDestroy(ppi->keyClick);
#endif
    deviceManagerUnregister(ppi->deviceHandle);
#ifndef TARGET_GNW
    debugDeviceUnregister(ppi->debugHandle);

    dacDestroy(ppi->dac);
#endif

    i8255Destroy(ppi->i8255);

#ifndef MSX_NO_MALLOC
    free(ppi);
#endif
}

static void reset(void* ppiv) 
{
    MsxPPI *ppi = (MsxPPI *)ppiv;
    ppi->row       = 0;
    ppi->regA   = -1;
    ppi->regCHi = -1;

    i8255Reset(ppi->i8255);
}

static void loadState(void* ppiv)
{
    MsxPPI *ppi = (MsxPPI *)ppiv;
    SaveState* state = saveStateOpenForRead("MsxPPI");

    ppi->row    = (UInt8)saveStateGet(state, "row", 0);
    ppi->regA   =        saveStateGet(state, "regA", -1);
    ppi->regCHi =        saveStateGet(state, "regCHi", -1);

    saveStateClose(state);
    
    i8255LoadState(ppi->i8255);
}

static void saveState(void* ppiv)
{
    MsxPPI *ppi = (MsxPPI *)ppiv;
    SaveState* state = saveStateOpenForWrite("MsxPPI");
    
    saveStateSet(state, "row", ppi->row);
    saveStateSet(state, "regA", ppi->regA);
    saveStateSet(state, "regCHi", ppi->regCHi);

    saveStateClose(state);

    i8255SaveState(ppi->i8255);
}

static void writeA(void* ppiv, UInt8 value)
{
    MsxPPI *ppi = (MsxPPI *)ppiv;
    if (value != ppi->regA) {
        int i;

        ppi->regA = value;

        for (i = 0; i < 4; i++) {
            slotSetRamSlot(i, value & 3);
            value >>= 2;
        }
    }
}

static void writeCLo(void* ppi, UInt8 value)
{
    ((MsxPPI *)ppi)->row = value;
}

static void writeCHi(void* ppiv, UInt8 value)
{
    MsxPPI *ppi = (MsxPPI *)ppiv;
    if (value != ppi->regCHi) {
        ppi->regCHi = value;

#ifndef TARGET_GNW
        audioKeyClick(ppi->keyClick, value & 0x08);
        dacWrite(ppi->dac, DAC_CH_MONO, (value & 0x02) ? 0 : 255);
        ledSetCapslock(!(value & 0x04));
#endif
    }
}

#ifndef TARGET_GNW
static UInt8 peekB(MsxPPI* ppi)
{
    UInt8 value = getKeyState(ppi->row);

    if (ppi->row == 8) {
        int renshaSpeed = switchGetRensha();
        if (renshaSpeed) {
            UInt8 renshaOn = (UInt8)((UInt64)renshaSpeed * boardSystemTime() / boardFrequency());
            value |= (renshaOn & 1);
        }
    }

    return value;
}
#endif

static UInt8 readB(void* ppiv)
{
#ifndef TARGET_GNW
    MsxPPI *ppi = (MsxPPI *)ppiv;
    UInt8 value = boardCaptureUInt8(ppi->row, getKeyState(ppi->row));

    if (ppi->row == 8) {
        int renshaSpeed = switchGetRensha();
        if (renshaSpeed) {
            UInt8 renshaOn = (UInt8)((UInt64)renshaSpeed * boardSystemTime() / boardFrequency());
            ledSetRensha(renshaSpeed > 14 ? 1 : renshaOn & 2);
            value |= (renshaOn & 1);
        }
        else {
            ledSetRensha(0);
        }
    }

    return value;
#else
    return getKeyState(((MsxPPI *)ppiv)->row);
#endif
}

#ifndef TARGET_GNW
static void getDebugInfo(MsxPPI* ppi, DbgDevice* dbgDevice)
{
    DbgIoPorts* ioPorts;

    ioPorts = dbgDeviceAddIoPorts(dbgDevice, langDbgDevPpi(), 4);
    dbgIoPortsAddPort(ioPorts, 0, 0xa8, DBG_IO_READWRITE, i8255Peek(ppi->i8255, 0xa8));
    dbgIoPortsAddPort(ioPorts, 1, 0xa9, DBG_IO_READWRITE, i8255Peek(ppi->i8255, 0xa9));
    dbgIoPortsAddPort(ioPorts, 2, 0xaa, DBG_IO_READWRITE, i8255Peek(ppi->i8255, 0xaa));
    dbgIoPortsAddPort(ioPorts, 3, 0xab, DBG_IO_READWRITE, i8255Peek(ppi->i8255, 0xab));
}
#endif

void msxPPICreate(int ignoreKeyboard)
{
    DeviceCallbacks callbacks = { destroy, reset, saveState, loadState };
#ifndef TARGET_GNW
    DebugCallbacks dbgCallbacks = { getDebugInfo, NULL, NULL, NULL };
    MsxPPI* ppi = malloc(sizeof(MsxPPI));
#else
    MsxPPI* ppi = itc_malloc(sizeof(MsxPPI));
#endif

    ppi->deviceHandle = deviceManagerRegister(RAM_MAPPER, &callbacks, ppi);
#ifndef TARGET_GNW
    ppi->debugHandle = debugDeviceRegister(DBGTYPE_BIOS, langDbgDevPpi(), &dbgCallbacks, ppi);
#endif

    if (ignoreKeyboard) {
        ppi->i8255 = i8255Create(NULL,  NULL,  writeA,
                                 NULL,  NULL,  NULL,
                                 NULL,  NULL,  writeCLo,
                                 NULL,  NULL,  writeCHi,
                                 ppi);
    }
    else {
        ppi->i8255 = i8255Create(NULL,  NULL,  writeA,
#ifndef TARGET_GNW
                                 peekB, readB, NULL,
#else
                                 NULL, readB, NULL,
#endif
                                 NULL,  NULL,  writeCLo,
                                 NULL,  NULL,  writeCHi,
                                 ppi);
    }
#ifndef TARGET_GNW
    ppi->keyClick = audioKeyClickCreate(boardGetMixer());

    ppi->dac = dacCreate(boardGetMixer(), DAC_MONO);
#endif

    ioPortRegister(0xa8, i8255Read, i8255Write, ppi->i8255);
    ioPortRegister(0xa9, i8255Read, i8255Write, ppi->i8255);
    ioPortRegister(0xaa, i8255Read, i8255Write, ppi->i8255);
    ioPortRegister(0xab, i8255Read, i8255Write, ppi->i8255);

    reset(ppi);
}



static UInt8 getKeyState(int row)
{
	#define _ROW(k7,k6,k5,k4,k3,k2,k1,k0) ((inputEventGetState(k7)<<7)|(inputEventGetState(k6)<<6)|(inputEventGetState(k5)<<5)|(inputEventGetState(k4)<<4)|(inputEventGetState(k3)<<3)|(inputEventGetState(k2)<<2)|(inputEventGetState(k1)<<1)|inputEventGetState(k0))
	#define ROW0  ~_ROW(EC_7,      EC_6,      EC_5,      EC_4,      EC_3,      EC_2,      EC_1,      EC_0      )
	#define ROW1  ~_ROW(EC_SEMICOL,EC_LBRACK, EC_AT,     EC_BKSLASH,EC_CIRCFLX,EC_NEG,    EC_9,      EC_8      )
	#define ROW2  ~_ROW(EC_B,      EC_A,      EC_UNDSCRE,EC_DIV,    EC_PERIOD, EC_COMMA,  EC_RBRACK, EC_COLON  )
	#define ROW3  ~_ROW(EC_J,      EC_I,      EC_H,      EC_G,      EC_F,      EC_E,      EC_D,      EC_C      )
	#define ROW4  ~_ROW(EC_R,      EC_Q,      EC_P,      EC_O,      EC_N,      EC_M,      EC_L,      EC_K      )
	#define ROW5  ~_ROW(EC_Z,      EC_Y,      EC_X,      EC_W,      EC_V,      EC_U,      EC_T,      EC_S      )
	#define ROW6 ~(_ROW(EC_F3,     EC_F2,     EC_F1,     EC_CODE,   EC_CAPS,   EC_GRAPH,  EC_CTRL,   EC_LSHIFT )|inputEventGetState(EC_RSHIFT))
	#define ROW7  ~_ROW(EC_RETURN, EC_SELECT, EC_BKSPACE,EC_STOP,   EC_TAB,    EC_ESC,    EC_F5,     EC_F4     )
	#define ROW8  ~_ROW(EC_RIGHT,  EC_DOWN,   EC_UP,     EC_LEFT,   EC_DEL,    EC_INS,    EC_CLS,    EC_SPACE  )
	#define ROW9  ~_ROW(EC_NUM4,   EC_NUM3,   EC_NUM2,   EC_NUM1,   EC_NUM0,   EC_NUMDIV, EC_NUMADD, EC_NUMMUL )
	#define ROW10 ~_ROW(EC_NUMPER, EC_NUMCOM, EC_NUMSUB, EC_NUM9,   EC_NUM8,   EC_NUM7,   EC_NUM6,   EC_NUM5   )
	#define ROW11 ~((inputEventGetState(EC_TORIKE)<<3)|(inputEventGetState(EC_JIKKOU)<<1))

#ifndef TARGET_GNW
    Properties* pProperties = propGetGlobalProperties();
    if (!pProperties->keyboard.enableKeyboardQuirk)
    {
#endif
	    switch (row) {
		    case 0:  return ROW0;
		    case 1:  return ROW1;
		    case 2:  return ROW2;
		    case 3:  return ROW3;
		    case 4:  return ROW4;
		    case 5:  return ROW5;
		    case 6:  return ROW6;
		    case 7:  return ROW7;
		    case 8:  return ROW8;
		    case 9:  return ROW9;
		    case 10: return ROW10;
		    case 11: return ROW11;
		    default: break;
	    }
	    return 0xff;
#ifndef TARGET_GNW
    }
    else {
        /*
	    Same, but including MSX keyboard matrix quirk, eg. pressing X+Z+J results in X+Z+H+J.
	    Slower than the above, since it needs data of all rows
	    */
	    UInt8 keyrow[12]={ROW0,ROW1,ROW2,ROW3,ROW4,ROW5,ROW6,ROW7,ROW8,ROW9,ROW10,ROW11};
	    int i=11;
	
	    if (row>11) return 0xff;
	
	    while (i--) {
		
		    if (keyrow[i]==0xff) continue;
		
		    if (i==6) {
			    /* modifier keys */
			    int k=keyrow[6]&0x15; keyrow[6]|=0x15;
			    if (keyrow[6]!=0xff) {
				    int j=12;
				    while (j--) {
					    if ((keyrow[6]|keyrow[j])!=0xff) keyrow[6]=keyrow[j]=keyrow[6]&keyrow[j];
				    }
			    }
			    keyrow[6]=k|(keyrow[6]&0xea);
		    }
		
		    else {
			    int j=12;
			    while (j--) {
				    if ((keyrow[i]|keyrow[j])!=0xff) keyrow[i]=keyrow[j]=keyrow[i]&keyrow[j];
			    }
		    }
	    }
	
	    return keyrow[row];
    }
#endif
}
#endif
