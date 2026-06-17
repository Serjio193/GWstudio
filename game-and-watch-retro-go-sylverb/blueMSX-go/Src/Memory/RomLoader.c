/*****************************************************************************
** $Source: /cygdrive/d/Private/_SVNROOT/bluemsx/blueMSX/Src/Memory/RomLoader.c,v $
**
** $Revision: 1.6 $
**
** $Date: 2008-03-30 18:38:42 $
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
#include "RomLoader.h"
#ifndef TARGET_GNW
#include "ziphelper.h"
#else
#include "rom_manager.h"
#include "main_msx.h"
#endif
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

// PacketFileSystem.h Need to be included after all other includes
#include "PacketFileSystem.h"

UInt8* romLoad(const char *fileName, const char *fileInZipFile, int* size)
{
#if !defined(TARGET_GNW) || defined (LINUX_EMU)
    UInt8* buf = NULL;
    FILE *file;

    if (fileName == NULL || strlen(fileName) == 0) {
        goto error;
    }

#ifndef LINUX_EMU
    if (fileInZipFile != NULL && strlen(fileInZipFile) == 0) {
        fileInZipFile = NULL;
    }

    if (fileInZipFile != NULL) {
        buf = zipLoadFile(fileName, fileInZipFile, size);
        if (buf == NULL)
           goto error;
        return buf;
    }
#endif

    file = fopen(fileName, "rb");
    if (file == NULL) {
        goto error;
    }

    fseek(file, 0, SEEK_END);
    *size = ftell(file);
    if (*size == 0) {
        fclose(file);
        return malloc(1);
    }

    fseek(file, 0, SEEK_SET);

    buf = malloc(*size);
    
    *size = fread(buf, 1, *size, file);
    fclose(file);

    return buf;

error:
#ifdef LINUX_EMU
    printf("!!!!!!! %s file not found\n",fileName);
#endif
    if (fileName && fileName[0])
      fflush(stdout);
    return NULL;
#else
    uint8_t *rom_data;
    retro_emulator_file_t *rom_file;

    rom_system_t *rom_system = (rom_system_t *)rom_manager_system(&rom_mgr, "MSX_BIOS");
    rom_file = (retro_emulator_file_t *)rom_manager_get_file((const rom_system_t *)rom_system,fileName);
    if (rom_file == NULL) {
        rom_system = (rom_system_t *)rom_manager_system(&rom_mgr, "MSX");
        rom_file = (retro_emulator_file_t *)rom_manager_get_file((const rom_system_t *)rom_system,fileName);
    }
    if (rom_file == NULL) {
        printf("%s rom not found\n",fileName);
        return NULL;
    }

    // Decompress rom if needed
    *size = msx_getromdata(&rom_data,
                           (uint8_t *)rom_file->address,
                           rom_file->size,
                           rom_file->ext);

    return (UInt8*)rom_data;
#endif
}

#endif
