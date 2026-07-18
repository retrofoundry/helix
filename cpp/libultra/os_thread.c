#include <ultra64.h>
#include "macros.h"
#include <helix/runtime.h>

/* Runtime is already bootstrapped by HLXRuntimeInit() in host_main.c. */
void osInitialize(void) {
}

void osCreateThread(OSThread *thread, OSId id, void (*entry)(void *), void *arg,
                    UNUSED void *sp, OSPri pri) {
    HLXThreadCreate(thread, id, entry, arg, sp, pri);
}

void osStartThread(OSThread *thread) {
    HLXThreadStart(thread);
}

void osSetThreadPri(OSThread *thread, OSPri pri) {
    HLXThreadSetPri(thread, pri);
}

void osStopThread(OSThread *thread) {
    HLXThreadStop(thread);
}

/* TLB + cache are meaningless on the native-pointer runtime. */
void osMapTLB(UNUSED s32 index, UNUSED OSPageMask mask, UNUSED void *vaddr,
              UNUSED u32 odd, UNUSED u32 even, UNUSED s32 asid) {
}
void osUnmapTLBAll(void) {
}
void osWritebackDCacheAll(void) {
}
void osWritebackDCache(UNUSED void *addr, UNUSED size_t nbytes) {
}
void osInvalDCache(UNUSED void *addr, UNUSED size_t nbytes) {
}
