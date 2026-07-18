#include <stdint.h>
#include <ultra64.h>
#include <PR/abi.h>
#include "mixer.h"
#include "acmd.h"

void hlx_acmd_process(const void *list, int32_t num_cmds) {
    const Acmd *a = (const Acmd *)list;
    for (int32_t i = 0; i < num_cmds; i++, a++) {
        uintptr_t w0 = a->words.w0;
        uintptr_t w1 = a->words.w1;
        uint32_t cmd   = (uint32_t)((w0 >> 24) & 0xff);
        uint8_t  flags = (uint8_t)((w0 >> 16) & 0xff);
        switch (cmd) {
        case A_SPNOOP:
        case A_SEGMENT: // native pointers: segment table unused (aSegment(0,0))
            break;
        case A_CLEARBUFF:
            aClearBufferImpl((uint16_t)(w0 & 0xffff), (int)(uint32_t)(w1 & 0xffff));
            break;
        case A_SETBUFF:
            aSetBufferImpl(flags, (uint16_t)(w0 & 0xffff),
                           (uint16_t)((w1 >> 16) & 0xffff), (uint16_t)(w1 & 0xffff));
            break;
        case A_DMEMMOVE:
            aDMEMMoveImpl((uint16_t)(w0 & 0xffff), (uint16_t)((w1 >> 16) & 0xffff),
                          (int)(uint32_t)(w1 & 0xffff));
            break;
        case A_LOADADPCM:
            aLoadADPCMImpl((int)(uint32_t)(w0 & 0xffffff), (const int16_t *)(uintptr_t)w1);
            break;
        case A_SETLOOP:
            aSetLoopImpl((ADPCM_STATE *)(uintptr_t)w1);
            break;
        case A_ADPCM:
            aADPCMdecImpl(flags, (int16_t *)(uintptr_t)w1);
            break;
        case A_RESAMPLE:
            aResampleImpl(flags, (uint16_t)(w0 & 0xffff), (int16_t *)(uintptr_t)w1);
            break;
        case A_SETVOL: // also handles aSetVolume32 (t=hi16, r=lo16 of w1)
            aSetVolumeImpl(flags, (int16_t)(w0 & 0xffff),
                           (int16_t)((w1 >> 16) & 0xffff), (int16_t)(w1 & 0xffff));
            break;
        case A_ENVMIXER:
            aEnvMixerImpl(flags, (int16_t *)(uintptr_t)w1);
            break;
        case A_LOADBUFF:
            aLoadBufferImpl((const void *)(uintptr_t)w1);
            break;
        case A_SAVEBUFF:
            aSaveBufferImpl((int16_t *)(uintptr_t)w1);
            break;
        case A_MIXER:
            aMixImpl((int16_t)(w0 & 0xffff), (uint16_t)((w1 >> 16) & 0xffff),
                     (uint16_t)(w1 & 0xffff));
            break;
        case A_INTERLEAVE:
            aInterleaveImpl((uint16_t)((w1 >> 16) & 0xffff), (uint16_t)(w1 & 0xffff));
            break;
        default:
            break; // A_LOADBUFF/A_SAVEBUFF SH variants + A_POLEF unused by US
        }
    }
}
