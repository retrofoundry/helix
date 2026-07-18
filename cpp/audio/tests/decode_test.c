#include <ultra64.h>
#include <PR/abi.h>
#include <string.h>
#include <stdio.h>
#include "acmd.h"

int main(void) {
    // Scenario: clear 32 DMEM bytes at 0x40, set nbytes=0x20/out=0x40, save to dst.
    Acmd list[8]; Acmd *cmd = list;
    int16_t dst[16];
    memset(dst, 0x7f, sizeof(dst));
    aClearBuffer(cmd++, 0x40, 0x20);
    aSetBuffer(cmd++, 0, 0, 0x40, 0x20);
    aSaveBuffer(cmd++, dst);
    hlx_acmd_process(list, (int32_t)(cmd - list));
    for (int i = 0; i < 16; i++) {
        if (dst[i] != 0) { printf("DECODE FAIL at %d = %d\n", i, dst[i]); return 1; }
    }
    printf("DECODE OK\n");
    return 0;
}
