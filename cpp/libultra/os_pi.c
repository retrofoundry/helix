#include <libultra/os_pi.h>
#include <helix/internal.h>

// osVirtualToPhysical MUST preserve full pointer width. include/PR/os.h declares a
// u32 return that would truncate 64-bit host pointers (goddard embeds them into DLs); we rely on
// include/ultra64.h pulling only PR/os_misc.h's uintptr_t decl. Fail the build if that ever
// regresses to a narrower integer.
_Static_assert(sizeof(uintptr_t) >= sizeof(void *),
               "osVirtualToPhysical would truncate host pointers");
uintptr_t osVirtualToPhysical(void *addr) {
    // Identity map — helix runs guest pointers directly in the host address space.
    // Re-homed here (not dropped by this rewrite); the old src/pc/ultra_reimplementation.c
    // copy is gone, so this is the only definition.
    return (uintptr_t) addr;
}

s32 osPiStartDma(OSIoMesg *mb, __attribute__((unused)) s32 priority, s32 direction,
                 uintptr_t devAddr, void *vAddr, size_t nbytes, OSMesgQueue *mq) {
    return HLXPiStartDma((void *) mb, direction, (size_t) devAddr, vAddr, nbytes, (void *) mq);
}

void osCreatePiManager(__attribute__((unused)) s32 pri, __attribute__((unused)) OSMesgQueue *cmdQ,
                        __attribute__((unused)) OSMesg *cmdBuf, __attribute__((unused)) s32 cmdMsgCnt) {
    // Synchronous DMA (HLXPiStartDma) needs no PI worker thread.
}
