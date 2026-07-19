#include <libultra/os_vi.h>   // OSViMode, TV_TYPE_*, osTvType/osViModeTable, osVi* decls

#include <helix/internal.h>

// "No VI mode override" sentinel handed to HLXViSetModeIndex; must equal helix's
// VI_MODE_UNSET (u32::MAX) so the retrace rate falls back to osTvType.
#define HLX_VI_MODE_UNSET 0xFFFFFFFFu

extern void HLXViSetEvent(void *mq, void *msg, u32 retrace);
extern void HLXViSwapBuffer(void *fb);

// osTvType lived in the deleted src/pc/ultra_reimplementation.c's sibling os_vi.c
// stub — re-home it here. thread1_idle (src/game/main.c) reads it
// (VERSION_US/VERSION_SH) to pick NTSC vs PAL before calling osViSetMode.
u32 osTvType = TV_TYPE_NTSC; /* US */

// VI ROM mode data on real hardware. main.c only takes &osViModeTable[mode] and
// hands it to the no-op osViSetMode, so a zeroed table just needs to exist to link.
OSViMode osViModeTable[56];

void osCreateViManager(OSPri pri) {
}

void osViSetMode(OSViMode *mode) {
    // Hand helix the mode's index in osViModeTable so the retrace clock paces at the
    // matching TV family's refresh rate. A null or out-of-table pointer clears the
    // override (helix falls back to osTvType). Validate via uintptr_t arithmetic
    // (implementation-defined) instead of relational pointer comparison against the table
    // bounds (which is UB when `mode` isn't a pointer into osViModeTable), and require the
    // pointer to land exactly on a table element.
    u32 index;
    if (mode == NULL) {
        index = HLX_VI_MODE_UNSET;
    } else {
        uintptr_t base = (uintptr_t) &osViModeTable[0];
        uintptr_t p = (uintptr_t) mode;
        uintptr_t span = (uintptr_t) 56 * sizeof(OSViMode);
        if (p >= base && p < base + span && (p - base) % sizeof(OSViMode) == 0) {
            index = (u32) ((p - base) / sizeof(OSViMode));
        } else {
            index = HLX_VI_MODE_UNSET;
        }
    }
    HLXViSetModeIndex(index);
}

void osViBlack(u8 active) {
}

void osViSetSpecialFeatures(u32 func) {
}

void osViSetEvent(OSMesgQueue *mq, OSMesg msg, u32 retraceCount) {
    HLXViSetEvent((void *) mq, (void *) msg, retraceCount);
}

void osViSwapBuffer(void *vaddr) {
    HLXViSwapBuffer(vaddr);
}
