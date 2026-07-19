#ifndef HELIX_LIBULTRA_OS_VI_H
#define HELIX_LIBULTRA_OS_VI_H

// libultra Video Interface (VI) ABI — helix's canonical, guest-agnostic definition.
// OSViMode's layout must match the guest's byte-for-byte: the guest computes
// &osViModeTable[mode] and the shim recovers the index via sizeof(OSViMode) (os_vi.c).

#include <libultra/ultratypes.h>
#include <libultra/os_thread.h>   // OSPri
#include <libultra/os_message.h>  // OSMesgQueue, OSMesg

// TV family (PR/libultra.h).
#define TV_TYPE_PAL  0
#define TV_TYPE_NTSC 1
#define TV_TYPE_MPAL 2

typedef struct {
    u32 ctrl;
    u32 width;
    u32 burst;
    u32 vSync;
    u32 hSync;
    u32 leap;
    u32 hStart;
    u32 xScale;
    u32 vCurrent;
} OSViCommonRegs;

typedef struct {
    u32 origin;
    u32 yScale;
    u32 vStart;
    u32 vBurst;
    u32 vIntr;
} OSViFieldRegs;

typedef struct {
    u8 type;
    OSViCommonRegs comRegs;
    OSViFieldRegs fldRegs[2];
} OSViMode;

extern u32 osTvType;
extern OSViMode osViModeTable[56];

void osCreateViManager(OSPri pri);
void osViSetMode(OSViMode *mode);
void osViBlack(u8 active);
void osViSetSpecialFeatures(u32 func);
void osViSetEvent(OSMesgQueue *mq, OSMesg msg, u32 retraceCount);
void osViSwapBuffer(void *vaddr);

#endif
