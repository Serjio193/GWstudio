/*****************************************************************************
** $Source: /cvsroot/bluemsx/blueMSX/Src/SoundChips/MsxPsg.c,v $
**
** $Revision: 1.15 $
**
** $Date: 2008/09/09 04:40:32 $
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
#include "MsxPsg.h"
#include "Board.h"
#include "AY8910.h"
#include "JoystickPort.h"
#include "SaveState.h"
#include "DeviceManager.h"
#include "Led.h"
#include "Switches.h"
#include "Casette.h"

#include "MsxJoystickDevice.h"
#include "MsxJoystick.h"
#ifndef TARGET_GNW
#include "DAC.h"
#include "MsxGunstick.h"
#include "MsxAsciiLaser.h"
#include "MsxMouse.h"
#else
#include "gw_malloc.h"
#endif
#include "Board.h"
#include "MsxTetrisDongle.h"
#include "MagicKeyDongle.h"
#include "MsxArkanoidPad.h"

#include <stdlib.h>

struct MsxPsg {
    int deviceHandle;
    AY8910* ay8910;
    int currentPort;
    int maxPorts;
#ifndef TARGET_GNW
    CassetteCb casCb;
    void*      casRef;
#endif
    UInt8 registers[2];
    UInt8 readValue[2];
    MsxJoystickDevice* devFun[2];
#ifndef TARGET_GNW
    DAC*   dac;
#endif
};

static void joystickPortHandler(MsxPsg* msxPsg, int port, JoystickPortType type)
{
    if (port >= msxPsg->maxPorts) {
        return;
    }

    if (msxPsg->devFun[port] != NULL && msxPsg->devFun[port]->destroy != NULL) {
        msxPsg->devFun[port]->destroy(msxPsg->devFun[port]);
    }

    switch (type) {
    default:
    case JOYSTICK_PORT_NONE:
        msxPsg->devFun[port] = NULL;
        break;
#ifndef TARGET_GNW
#ifndef EXCLUDE_JOYSTICK_PORT_GUNSTICK
    case JOYSTICK_PORT_GUNSTICK:
        msxPsg->devFun[port] = msxGunstickCreate();
        break;
#endif
#ifndef EXCLUDE_JOYSTICK_PORT_ASCIILASER
    case JOYSTICK_PORT_ASCIILASER:
        msxPsg->devFun[port] = msxAsciiLaserCreate();
        break;
#endif
#ifndef EXCLUDE_JOYSTICK_PORT_JOYSTICK
    case JOYSTICK_PORT_JOYSTICK:
        msxPsg->devFun[port] = msxJoystickCreate(port);
        break;
#endif
#ifndef EXCLUDE_JOYSTICK_PORT_MOUSE
    case JOYSTICK_PORT_MOUSE:
        msxPsg->devFun[port] = msxMouseCreate();
        break;
#endif
#ifndef EXCLUDE_JOYSTICK_PORT_TETRIS2DONGLE
    case JOYSTICK_PORT_TETRIS2DONGLE:
        msxPsg->devFun[port] = msxTetrisDongleCreate();
        break;
#endif
#ifndef EXCLUDE_JOYSTICK_PORT_MAGICKEYDONGLE
    case JOYSTICK_PORT_MAGICKEYDONGLE:
        msxPsg->devFun[port] = magicKeyDongleCreate();
        break;
#endif
#ifndef EXCLUDE_JOYSTICK_PORT_ARKANOID_PAD
    case JOYSTICK_PORT_ARKANOID_PAD:
        msxPsg->devFun[port] = msxArkanoidPadCreate();
        break;
#endif
#else // TARGET_GNW
    case JOYSTICK_PORT_JOYSTICK:
        msxPsg->devFun[port] = msxJoystickCreate(port);
        break;
#endif
    }
}

#ifndef TARGET_GNW
static UInt8 peek(MsxPsg* msxPsg, UInt16 address)
{
    if (address & 1) {
        /* r15 */
        return msxPsg->registers[1];
    }
    
    /* r14 */
    else return msxPsg->readValue[address & 1];
}
#endif

static UInt8 read(void* msxPsgv, UInt16 address)
{
//    UInt8 casdat = 0;
    MsxPsg *msxPsg = (MsxPsg *)msxPsgv;

    if (address & 1) {
    	/* r15 */
        return msxPsg->registers[1];
    }
    else {
        /* r14 */
        /* joystick pins */
	    UInt8 state = 0x3f;
#ifndef TARGET_GNW
        int renshaSpeed = switchGetRensha();
#endif
        if (msxPsg->devFun[msxPsg->currentPort] != NULL &&
            msxPsg->devFun[msxPsg->currentPort]->read != NULL) {
            state = msxPsg->devFun[msxPsg->currentPort]->read(msxPsg->devFun[msxPsg->currentPort]);
        }
#ifndef TARGET_GNW
        state = boardCaptureUInt8(16 + msxPsg->currentPort, state);
        if (renshaSpeed) {
            state &= ~((((UInt64)renshaSpeed * boardSystemTime() / boardFrequency()) & 1) << 4);
        }
#endif
        /* pins 6/7 input ANDed with pins 6/7 output */
        state&=((msxPsg->registers[1]>>(msxPsg->currentPort<<1&2)&3)<<4|0xf);
        
        /* ANSI/JIS */
        state |= 0x40;
        
#ifndef TARGET_GNW
        /* cas signal */
        // Call cassette Callback (for coin select
        if (msxPsg->casCb != NULL && msxPsg->casCb(msxPsg->casRef)) {
            state |= 0x80;
        }
#endif

#if 0
        // COmment out until cassette wave is working
        tapeRead(&casdat);
        state |= (casdat) ? 0:0x80;
       	dacWrite(msxPsg->dac, DAC_CH_MONO, (casdat) ? 0 : 255);
#endif
        msxPsg->readValue[address & 1] = state;

        return state;
    }
}

static void write(void* msxPsgv, UInt16 address, UInt8 value)
{
    MsxPsg *msxPsg = (MsxPsg *)msxPsgv;
    if (address & 1) {
        /* r15 */
        if (msxPsg->devFun[0] != NULL && msxPsg->devFun[0]->write != NULL) {
	        UInt8 val = ((value >> 0) & 0x03) | ((value >> 2) & 0x04);
	        msxPsg->devFun[0]->write(msxPsg->devFun[0], val);
        }
        if (msxPsg->devFun[1] != NULL && msxPsg->devFun[1]->write != NULL) {
	        UInt8 val = ((value >> 2) & 0x03) | ((value >> 3) & 0x04);
	        msxPsg->devFun[1]->write(msxPsg->devFun[1], val);
        }

	    msxPsg->currentPort = (value >> 6) & 0x01;

#ifndef TARGET_GNW
        ledSetKana(0 == (value & 0x80));
#endif
    }
    msxPsg->registers[address & 1] = value;
}

static void saveState(void* msxPsgv)
{
    MsxPsg *msxPsg = (MsxPsg *)msxPsgv;
    // dink: fix for certain games where buttons are stuck after reload of state
    SaveState* state = saveStateOpenForWrite("MsxPsg");
    saveStateSet(state, "currentport", msxPsg->currentPort);
    saveStateSet(state, "registers0", msxPsg->registers[0]);
    saveStateSet(state, "registers1", msxPsg->registers[1]);
    saveStateClose(state);

    if (msxPsg->devFun[0] != NULL && msxPsg->devFun[0]->saveState != NULL) {
	    msxPsg->devFun[0]->saveState(msxPsg->devFun[0]);
    }
    if (msxPsg->devFun[1] != NULL && msxPsg->devFun[1]->saveState != NULL) {
	    msxPsg->devFun[1]->saveState(msxPsg->devFun[1]);
    }

    ay8910SaveState(msxPsg->ay8910);
}

static void loadState(void* msxPsgv)
{
    MsxPsg *msxPsg = (MsxPsg *)msxPsgv;
    // dink: fix for certain games where buttons are stuck after reload of state
    SaveState* state = saveStateOpenForRead("MsxPsg");
    msxPsg->currentPort =          saveStateGet(state, "currentport", 0);
    msxPsg->registers[0] = (UInt8) saveStateGet(state, "registers0", 0);
    msxPsg->registers[1] = (UInt8) saveStateGet(state, "registers1", 0);
    saveStateClose(state);

    if (msxPsg->devFun[0] != NULL && msxPsg->devFun[0]->loadState != NULL) {
	    msxPsg->devFun[0]->loadState(msxPsg->devFun[0]);
    }
    if (msxPsg->devFun[1] != NULL && msxPsg->devFun[1]->loadState != NULL) {
	    msxPsg->devFun[1]->loadState(msxPsg->devFun[1]);
    }

    ay8910LoadState(msxPsg->ay8910);
}

static void reset(void* msxPsgv)
{
    MsxPsg *msxPsg = (MsxPsg *)msxPsgv;
    msxPsg->currentPort  = 0;
    msxPsg->registers[0] = 0;
    msxPsg->registers[1] = 0;
    msxPsg->readValue[0] = 0;
    msxPsg->readValue[1] = 0;

    if (msxPsg->devFun[0] != NULL && msxPsg->devFun[0]->reset != NULL) {
	    msxPsg->devFun[0]->reset(msxPsg->devFun[0]);
    }
    if (msxPsg->devFun[1] != NULL && msxPsg->devFun[1]->reset != NULL) {
	    msxPsg->devFun[1]->reset(msxPsg->devFun[1]);
    }

    ay8910Reset(msxPsg->ay8910);
}

static void destroy(void* msxPsgv) 
{
    MsxPsg *msxPsg = (MsxPsg *)msxPsgv;
    ay8910SetIoPort(msxPsg->ay8910, NULL, NULL, NULL, NULL);
    ay8910Destroy(msxPsg->ay8910);

#ifndef TARGET_GNW
    joystickPortUpdateHandlerUnregister();
#endif
    deviceManagerUnregister(msxPsg->deviceHandle);
#ifndef TARGET_GNW
    dacDestroy(msxPsg->dac);
#endif
    if (msxPsg->devFun[0] != NULL && msxPsg->devFun[0]->destroy != NULL) {
	    msxPsg->devFun[0]->destroy(msxPsg->devFun[0]);
    }
    if (msxPsg->devFun[1] != NULL && msxPsg->devFun[1]->destroy != NULL) {
	    msxPsg->devFun[1]->destroy(msxPsg->devFun[1]);
    }
#ifndef MSX_NO_MALLOC
    free(msxPsg);
#endif
}

#ifndef TARGET_GNW
void msxPsgRegisterCassetteRead(MsxPsg* msxPsg, CassetteCb cb, void* ref)
{
    msxPsg->casCb  = cb;
    msxPsg->casRef = ref;
}
#endif

MsxPsg* msxPsgCreate(PsgType type, int stereo, int* pan, int maxPorts)
{
    DeviceCallbacks callbacks = { destroy, reset, saveState, loadState };
#ifndef TARGET_GNW
    MsxPsg* msxPsg = (MsxPsg*)calloc(1, sizeof(MsxPsg));
#else
    MsxPsg* msxPsg = (MsxPsg*)itc_calloc(1, sizeof(MsxPsg));
#endif

    msxPsg->ay8910 = ay8910Create(boardGetMixer(), AY8910_MSX, type, stereo, pan);
    msxPsg->maxPorts = maxPorts;

#ifndef TARGET_GNW
    msxPsg->dac = dacCreate(boardGetMixer(), DAC_MONO);
#endif

#ifndef TARGET_GNW
    ay8910SetIoPort(msxPsg->ay8910, read, peek, write, msxPsg);
#else
    ay8910SetIoPort(msxPsg->ay8910, read, NULL, write, msxPsg);
#endif

    joystickPortUpdateHandlerRegister((JoystickPortUpdateHandler)joystickPortHandler, msxPsg);

    msxPsg->deviceHandle = deviceManagerRegister(ROM_UNKNOWN, &callbacks, msxPsg);

    return msxPsg;
}
#endif
