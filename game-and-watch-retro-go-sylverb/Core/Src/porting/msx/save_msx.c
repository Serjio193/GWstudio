#include "build/config.h"
#ifdef ENABLE_EMULATOR_MSX

#include <odroid_system.h>

#include "main.h"
#include "appid.h"

#include "common.h"
#include "gw_linker.h"
#include "gw_flash.h"
#include "gw_lcd.h"
#include "main_msx.h"
#include "save_msx.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include "Board.h"

static char *headerString = "bMSX0000";

#define WORK_BLOCK_SIZE (256)
static int flashBlockOffset = 0;
static bool isLastFlashWrite = 0;

// This function fills 4kB blocks and writes them in flash when full
static void SaveFlashSaveData(UInt8 *dest, UInt8 *src, int size) {
    int blockNumber = 0;
    for (int i = 0; i < size; i++) {
        // We use msx_framebuffer as a temporary buffer
        msx_framebuffer[flashBlockOffset] = src[i];
        flashBlockOffset++;
        if ((flashBlockOffset == WORK_BLOCK_SIZE) || (isLastFlashWrite && (i == size-1))) {
            // Write block in flash
            int intDest = (int)dest+blockNumber*WORK_BLOCK_SIZE;
            intDest = intDest & ~(WORK_BLOCK_SIZE-1);
            unsigned char *newDest = (unsigned char *)intDest;
            OSPI_DisableMemoryMappedMode();
            OSPI_Program((uint32_t)newDest,(const uint8_t *)msx_framebuffer,WORK_BLOCK_SIZE);
            OSPI_EnableMemoryMappedMode();
            flashBlockOffset = 0;
            blockNumber++;
        }
    }
    if (size == 0) {
        // force writing last data block
        int intDest = (int)dest+blockNumber*WORK_BLOCK_SIZE;
        intDest = intDest & ~(WORK_BLOCK_SIZE-1);
        unsigned char *newDest = (unsigned char *)intDest;
        OSPI_DisableMemoryMappedMode();
        OSPI_Program((uint32_t)newDest,(const uint8_t *)msx_framebuffer,WORK_BLOCK_SIZE);
        OSPI_EnableMemoryMappedMode();
        flashBlockOffset = 0;
        blockNumber++;
    }
}

struct SaveStateSection {
    UInt32 tag;
    UInt32 offset;
};

// We have 31*8 Bytes available for sections info
// Do not increase this value without reserving
// another 256 bytes block for header
#define MAX_SECTIONS 31

struct SaveState {
    struct SaveStateSection sections[MAX_SECTIONS];
    UInt16 section;
    UInt32 allocSize;
    UInt32 size;
    UInt32 offset;
    UInt8 *buffer;
    char   fileName[64];
};

extern BoardInfo boardInfo;

static SaveState msxSaveState;

static UInt32 tagFromName(const char* tagName)
{
    UInt32 tag = 0;
    UInt32 mod = 1;

    while (*tagName) {
        mod *= 19219;
        tag += mod * *tagName++;
    }

    return tag;
}

/* Savestate functions */
UInt32 saveMsxState(UInt8 *destBuffer, UInt32 save_size) {
    // Convert mem mapped pointer to flash address
    uint32_t save_address = destBuffer - &__EXTFLASH_BASE__;
    // Fill context data
    msxSaveState.buffer = (UInt8 *)save_address;
    msxSaveState.offset = 0;
    msxSaveState.section = 0;

    // Erase flash memory
    store_erase((const UInt8 *)destBuffer, save_size);

    isLastFlashWrite = 0;
    // Reserve a page of 256 Bytes for the header info
    // We put 0xff to be able to update values at the end of process
    SaveFlashSaveData(msxSaveState.buffer,(UInt8 *)headerString,8);
    memset(msxSaveState.sections,0xff,sizeof(msxSaveState.sections[0])*MAX_SECTIONS);
    SaveFlashSaveData(msxSaveState.buffer+8,(UInt8 *)msxSaveState.sections,sizeof(msxSaveState.sections[0])*MAX_SECTIONS);
    msxSaveState.offset += 8+sizeof(msxSaveState.sections[0])*MAX_SECTIONS;
    // Start saving data
    boardSaveState("mem0",0);
    save_gnw_msx_data();

    // Write dummy data to force writing last block of data
    SaveFlashSaveData(msxSaveState.buffer+msxSaveState.offset,NULL,0);

    // Copy header data in the first 256 bytes block of flash
    SaveFlashSaveData(msxSaveState.buffer,(UInt8 *)headerString,8);
    isLastFlashWrite = true;
    SaveFlashSaveData(msxSaveState.buffer+8,(UInt8 *)msxSaveState.sections,sizeof(msxSaveState.sections[0])*MAX_SECTIONS);

    return msxSaveState.offset;
}

void saveStateCreateForWrite(const char* fileName)
{
    // Nothing to do, no filename on the G&W
}

void saveStateSet(SaveState* state, const char* tagName, UInt32 value)
{
    SaveFlashSaveData(state->buffer+state->offset,(UInt8 *)&value,sizeof(UInt32));
    state->offset+=sizeof(UInt32);
}

void saveStateSetBuffer(SaveState* state, const char* tagName, void* buffer, UInt32 length)
{
    SaveFlashSaveData(state->buffer + state->offset, (UInt8 *)buffer, length);
    state->offset += length;
}

SaveState* saveStateOpenForWrite(const char* fileName)
{
    // Update section
    msxSaveState.sections[msxSaveState.section].tag = tagFromName(fileName);
    msxSaveState.sections[msxSaveState.section].offset = msxSaveState.offset;
    msxSaveState.section++;
    return &msxSaveState;
}

void saveStateDestroy(void)
{
}

void saveStateClose(SaveState* state)
{
}

/* Loadstate functions */
bool initLoadMsxState(UInt8 *srcBuffer) {
    msxSaveState.offset = 0;
    // Check for header
    if (memcmp(headerString,srcBuffer,8) == 0) {
        msxSaveState.buffer = srcBuffer;
        // Copy sections header in structure
        memcpy(msxSaveState.sections,msxSaveState.buffer+8,sizeof(msxSaveState.sections[0])*MAX_SECTIONS);
        return true;
    }
    return false;
}

UInt32 loadMsxState(UInt8 *srcBuffer) {
    msxSaveState.offset = 0;
    // Check for header
    if (memcmp(headerString,srcBuffer,8) == 0) {
        msxSaveState.buffer = srcBuffer;
        // Copy sections header in structure
        memcpy(msxSaveState.sections,msxSaveState.buffer+8,sizeof(msxSaveState.sections[0])*MAX_SECTIONS);
        boardInfo.loadState();
        load_gnw_msx_data(srcBuffer);
    }
    return msxSaveState.offset;
}

SaveState* saveStateOpenForRead(const char* fileName)
{
    // find offset
    UInt32 tag = tagFromName(fileName);
    for (int i = 0; i<MAX_SECTIONS; i++) {
        if (msxSaveState.sections[i].tag == tag) {
            // Found tag
            msxSaveState.offset = msxSaveState.sections[i].offset;
            return &msxSaveState;
        }
    }

    return NULL;
}

UInt32 saveStateGet(SaveState* state, const char* tagName, UInt32 defValue)
{
    UInt32 value;

    memcpy(&value, state->buffer + state->offset, sizeof(UInt32));
    state->offset+=sizeof(UInt32);
    return value;
}

void saveStateGetBuffer(SaveState* state, const char* tagName, void* buffer, UInt32 length)
{
    memcpy(buffer, state->buffer + state->offset, length);
    state->offset += length;
}

void saveStateCreateForRead(const char* fileName)
{
}

#endif