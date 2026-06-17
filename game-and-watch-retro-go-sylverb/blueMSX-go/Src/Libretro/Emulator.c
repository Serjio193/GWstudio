/*****************************************************************************
** $Source: /cygdrive/d/Private/_SVNROOT/bluemsx/blueMSX/Src/Emulator/Emulator.c,v $
**
** $Revision: 1.67 $
**
** $Date: 2009-07-18 14:35:59 $
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
#include "Emulator.h"
#include "MsxTypes.h"
#include "Debugger.h"
#include "Board.h"
#include "FileHistory.h"
#include "Switches.h"
#include "Led.h"
#include "InputEvent.h"

#include "ArchThread.h"
#include "ArchEvent.h"
#include "ArchTimer.h"
#include "ArchSound.h"
#ifndef TARGET_GNW
#include "ArchMidi.h"
#endif

#include "JoystickPort.h"
#include "ArchInput.h"
#include "ArchDialog.h"
#include "ArchNotifications.h"
#include <math.h>
#include <string.h>


static int    emuExitFlag;
static UInt32 emuSysTime = 0;
static UInt32 emuFrequency = 3579545;
int           emuMaxSpeed = 0;
int           emuPlayReverse = 0;
int           emuMaxEmuSpeed = 0; // Max speed issued by emulation
static volatile int      emuSuspendFlag;
static volatile EmuState emuState = EMU_STOPPED;
static volatile int      emuSingleStep = 0;
static Properties* properties;
static Mixer* mixer;
static BoardDeviceInfo deviceInfo;
static Machine* machine;
static int lastScreenMode;

static int    enableSynchronousUpdate = 1;


extern BoardInfo boardInfo;


void emuEnableSynchronousUpdate(int enable)
{
    enableSynchronousUpdate = enable;
}

void emulatorInit(Properties* theProperties, Mixer* theMixer)
{
    properties = theProperties;
    mixer      = theMixer;
}

void emulatorExit()
{
    properties = NULL;
    mixer      = NULL;
}


EmuState emulatorGetState() {
    return emuState;
}

void emulatorSetState(EmuState state) {
#ifndef TARGET_GNW
    if (state == EMU_RUNNING)
        archMidiEnable(1);
    else
        archMidiEnable(0);

    if (state == EMU_STEP) {
        state = EMU_RUNNING;
        emuSingleStep = 1;
    }

    if (state == EMU_STEP_BACK) {
        EmuState oldState = state;
        state = EMU_RUNNING;
        if (!boardRewindOne()) {
            state = oldState;
        }
    }
#endif
    emuState = state;
}


#ifndef TARGET_GNW
static void getDeviceInfo(BoardDeviceInfo* di)
{
    int i;

    for (i = 0; i < PROP_MAX_CARTS; i++) {
        strcpy(properties->media.carts[i].fileName, di->carts[i].name);
#ifndef MSX_NO_ZIP
        strcpy(properties->media.carts[i].fileNameInZip, di->carts[i].inZipName);
        // Don't save rom type
        // properties->media.carts[i].type = di->carts[i].type;
        updateExtendedRomName(i, properties->media.carts[i].fileName, properties->media.carts[i].fileNameInZip);
#else
        updateExtendedRomName(i, properties->media.carts[i].fileName, NULL);
#endif
    }

    for (i = 0; i < PROP_MAX_DISKS; i++) {
        strcpy(properties->media.disks[i].fileName, di->disks[i].name);
#ifndef MSX_NO_ZIP
        strcpy(properties->media.disks[i].fileNameInZip, di->disks[i].inZipName);
        updateExtendedDiskName(i, properties->media.disks[i].fileName, properties->media.disks[i].fileNameInZip);
#else
        updateExtendedDiskName(i, properties->media.disks[i].fileName, NULL);
#endif
    }

    for (i = 0; i < PROP_MAX_TAPES; i++) {
        strcpy(properties->media.tapes[i].fileName, di->tapes[i].name);
#ifndef MSX_NO_ZIP
        strcpy(properties->media.tapes[i].fileNameInZip, di->tapes[i].inZipName);
        updateExtendedCasName(i, properties->media.tapes[i].fileName, properties->media.tapes[i].fileNameInZip);
#else
        updateExtendedCasName(i, properties->media.tapes[i].fileName, NULL);
#endif
    }

    properties->emulation.vdpSyncMode      = di->video.vdpSyncMode;

}
#endif

static void setDeviceInfo(BoardDeviceInfo* di)
{
    int i;

    for (i = 0; i < PROP_MAX_CARTS; i++) {
        di->carts[i].inserted =  strlen(properties->media.carts[i].fileName);
        di->carts[i].type = properties->media.carts[i].type;
        strcpy(di->carts[i].name, properties->media.carts[i].fileName);
#ifndef MSX_NO_ZIP
        strcpy(di->carts[i].inZipName, properties->media.carts[i].fileNameInZip);
#endif
    }

    for (i = 0; i < PROP_MAX_DISKS; i++) {
        di->disks[i].inserted =  strlen(properties->media.disks[i].fileName);
        strcpy(di->disks[i].name, properties->media.disks[i].fileName);
#ifndef MSX_NO_ZIP
        strcpy(di->disks[i].inZipName, properties->media.disks[i].fileNameInZip);
#endif
    }

    for (i = 0; i < PROP_MAX_TAPES; i++) {
        di->tapes[i].inserted =  strlen(properties->media.tapes[i].fileName);
        strcpy(di->tapes[i].name, properties->media.tapes[i].fileName);
#ifndef MSX_NO_ZIP
        strcpy(di->tapes[i].inZipName, properties->media.tapes[i].fileNameInZip);
#endif
    }

    di->video.vdpSyncMode = properties->emulation.vdpSyncMode;
}

#ifndef TARGET_GNW
void emulatorStart(const char* stateName) {
   int frequency;
   int success = 0;
   int reversePeriod = 0;
   int reverseBufferCnt = 0;

    archEmulationStartNotification();
    emulatorResume();

    emuExitFlag = 0;

    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MOONSOUND, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_YAMAHA_SFG, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MSXAUDIO, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MSXMUSIC, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_SCC, 1);


    properties->emulation.pauseSwitch = 0;
    switchSetPause(properties->emulation.pauseSwitch);

    machine = machineCreate(properties->emulation.machineName);

    if (machine == NULL) {
        archShowStartEmuFailDialog();
        archEmulationStopNotification();
        archEmulationStartFailure();
        emuState = EMU_STOPPED;
        return;
    }

    boardSetMachine(machine);


    setDeviceInfo(&deviceInfo);

    inputEventReset();

    archMidiEnable(1);

    emuState = EMU_PAUSED;

    emuState = EMU_RUNNING;

    emulatorSetFrequency(50 , &frequency);

    switchSetFront(properties->emulation.frontSwitch);
    switchSetPause(properties->emulation.pauseSwitch);
    switchSetAudio(properties->emulation.audioSwitch);

    if (properties->emulation.reverseEnable && properties->emulation.reverseMaxTime > 0) {
        reversePeriod = 50;
        reverseBufferCnt = properties->emulation.reverseMaxTime * 1000 / reversePeriod;
    }
    success = boardRun(machine,
                       &deviceInfo,
                       mixer,
                       NULL,
                       frequency,
                       0,
                       0,
                       NULL);
    if (!success) {
        archEmulationStopNotification();
        emuState = EMU_STOPPED;
        archEmulationStartFailure();
    }
}
#else
void emulatorStartMachine(const char* stateName, Machine *machine) {
   int frequency;
   int success = 0;

    emulatorResume();

    emuExitFlag = 0;

    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MSXMUSIC, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_SCC, 1);

    if (machine == NULL) {
        emuState = EMU_STOPPED;
        return;
    }

    boardSetMachine(machine);

    setDeviceInfo(&deviceInfo);

    inputEventReset();

    emuState = EMU_PAUSED;

    emuState = EMU_RUNNING;

    emulatorSetFrequency(50 , &frequency);

    success = boardRun(machine,
                       &deviceInfo,
                       mixer,
                       NULL,
                       frequency,
                       0,
                       0,
                       NULL);
    if (!success) {
        emuState = EMU_STOPPED;
    }
}
#endif

void emulatorStop() {
    if (emuState == EMU_STOPPED) {
        return;
    }

#ifndef TARGET_GNW
    debuggerNotifyEmulatorStop();
#endif

    emuState = EMU_STOPPED;

    emuExitFlag = 1;

#ifndef TARGET_GNW
    archMidiEnable(0);
#endif
    machineDestroy(machine);

    // Reset active indicators in mixer
#ifndef TARGET_GNW
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MOONSOUND, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_YAMAHA_SFG, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MSXAUDIO, 1);
#endif
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MSXMUSIC, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_SCC, 1);

#ifndef TARGET_GNW
    archEmulationStopNotification();
#endif

}



#ifndef TARGET_GNW
void emulatorSetFrequency(int logFrequency, int* frequency) {
    emuFrequency = (int)(3579545 * pow(2.0, (logFrequency - 50) / 15.0515));

    if (frequency != NULL) {
        *frequency  = emuFrequency;
    }

    boardSetFrequency(emuFrequency);
}
#else
void emulatorSetFrequency(int logFrequency, int* frequency) {
    emuFrequency = (int)3579545;

    if (frequency != NULL) {
        *frequency  = emuFrequency;
    }

    boardSetFrequency(emuFrequency);
}
#endif

void emulatorSuspend() {
    if (emuState == EMU_RUNNING) {
        emuState = EMU_SUSPENDED;
#ifndef TARGET_GNW
        archMidiEnable(0);
#endif
    }
}

void emulatorResume() {
    if (emuState == EMU_SUSPENDED) {
        emuSysTime = 0;
#ifndef TARGET_GNW
        archMidiEnable(1);
#endif
        emuState = EMU_RUNNING;
    }
}

int emulatorGetCurrentScreenMode()
{
    return lastScreenMode;
}

void emulatorRestart() {
#ifndef TARGET_GNW
    Machine* machine = machineCreate(properties->emulation.machineName);

    emulatorStop();
    if (machine != NULL) {
        boardSetMachine(machine);
        machineDestroy(machine);
    }
#endif
}

void emulatorRestartSound() {
    emulatorSuspend();
    archSoundDestroy();
    archSoundCreate(mixer, 44100, properties->sound.bufSize, properties->sound.stereo ? 2 : 1);
    emulatorResume();
}


void emulatorSetMaxSpeed(int enable) {
}

int  emulatorGetMaxSpeed() {
    return 0;
}

void emulatorPlayReverse(int enable)
{

}

void emulatorResetMixer() {
    // Reset active indicators in mixer
#ifndef TARGET_GNW
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MOONSOUND, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_YAMAHA_SFG, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MSXAUDIO, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_PCM, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_IO, 1);
#endif
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_MSXMUSIC, 1);
    mixerIsChannelTypeActive(mixer, MIXER_CHANNEL_SCC, 1);
}


void RefreshScreen(int screenMode) {

    lastScreenMode = screenMode;

}

#endif
