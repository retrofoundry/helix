#include <ultra64.h>
#include <PR/abi.h>                // list-build macros
#include <string.h>
#include <stdio.h>
#include "acmd.h"
#include "scenario_data.h"

void hlx_mixer_reset_state(void);  // test hook in mixer.c: zero the shared rspa

// Baked golden reference: the exact PCM the (now-retired) inline mixer produced for this
// scenario, frozen after the gate proved interpreter == inline mixer byte-for-byte.
// Makes the gate self-contained (no src/pc/mixer.h dependency) and an ongoing interpreter
// regression check.
static const int16_t GOLDEN_REF[72] = {
     0,     0,     0,     0,     0,     0,     0,     0,
     0,     0,     0,     0,     0,     0,     0,     0,
     0,    -9,     2,     2,     2,   -12,     2,    -1,
     3,     9,     3,    -4,     5,     6,     5,    -7,
    96,  -168,   120,     0,   120,   168,   120,   -48,
   144,   120,   144,   -96,   168,    72,   168,  -144,
 -3072, -3072, -3072,  -384, -2688,  2304, -2688, -1152,
 -2304,  1536, -2304, -1920, -1920,   768, -1920, -2688,
-24983, 32767,-23568,-12924,-19752, 28427,-18480,-25691,
};

#define GOLDEN_PCM_BYTES 144

int main(void) {
    static int16_t neu[512];
    memset(neu, 0, sizeof(neu));

    // Non-vacuous guard: the baked reference must contain real non-zero audio.
    int nonzero = 0;
    for (size_t i = 0; i < GOLDEN_PCM_BYTES / sizeof(int16_t); i++)
        if (GOLDEN_REF[i] != 0) nonzero++;
    if (nonzero == 0) {
        printf("GOLDEN VACUOUS: baked reference all-zero -- test proves nothing\n");
        return 1;
    }

    {                              // interpreter path over the scenario
        SCENARIO_DECLS
        int16_t *out_pcm = neu;
        static Acmd list[64];
        Acmd *cmd = list;
        #include "scenario.inc"
        int32_t n = (int32_t)(cmd - list);
        hlx_mixer_reset_state();
        hlx_acmd_process(list, n);
        if (memcmp(GOLDEN_REF, neu, GOLDEN_PCM_BYTES) != 0) {
            for (size_t i = 0; i < GOLDEN_PCM_BYTES / sizeof(int16_t); i++)
                if (GOLDEN_REF[i] != neu[i]) { printf("GOLDEN MISMATCH at [%zu]: ref=%d new=%d\n", i, GOLDEN_REF[i], neu[i]); break; }
            return 1;
        }
        printf("GOLDEN OK (%d cmds, %d PCM bytes match baked reference, %d/%d s16 non-zero)\n",
               n, GOLDEN_PCM_BYTES, nonzero, (int)(GOLDEN_PCM_BYTES / sizeof(int16_t)));
    }
    return 0;
}
