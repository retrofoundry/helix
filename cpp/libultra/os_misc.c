#include <libultra/ultra64.h>
#include <helix/internal.h>

// Helix owns scheduling, PI and SP; the low-level libultra entry points are inert link stubs.

typedef u32 OSIntMask; // libultra interrupt mask; helix has no hardware interrupts

u32 osMemSize = 0x00800000; // 8 MiB (expansion pak); the guest sizes its heap from this
u32 osResetType = 0;        // cold boot, never pre-NMI
s32 osCicId = 6105;         // NTSC CIC
s32 osAppNMIBuffer[16];

u32 osGetMemSize(void) {
    return osMemSize;
}
s32 osAfterPreNMI(void) {
    return 0; // never in the pre-NMI window
}

OSIntMask __OSGlobalIntMask = 0;
OSIntMask osGetIntMask(void) {
    return 0;
}
OSIntMask osSetIntMask(OSIntMask mask) {
    (void) mask;
    return 0;
}

// Native memory needs no explicit cache invalidation.
void osInvalICache(void *vaddr, s32 nbytes) {
    (void) vaddr;
    (void) nbytes;
}

// Report an idle RDP to guest polling loops.
u32 osDpGetStatus(void) {
    return 0;
}
void osDpSetStatus(u32 status) {
    (void) status;
}

OSPiHandle *__osPiTable = (OSPiHandle *) 0;
OSThread *__osRunQueue = (OSThread *) 0;
OSThread *__osActiveQueue = (OSThread *) 0;
u8 __osEventStateTab[16 * 8];

s32 __osDisableInt(void) {
    return 0;
}
void __osRestoreInt(s32 mask) {
    (void) mask;
}
u32 __osSpGetStatus(void) {
    return 0;
}
void __osSpSetStatus(u32 status) {
    (void) status;
}
u32 __osGetFpcCsr(void) {
    return 0;
}
u32 __osSetFpcCsr(u32 v) {
    (void) v;
    return 0;
}
void __osEnqueueThread(OSThread **queue, OSThread *thread) {
    (void) queue;
    (void) thread;
}
OSThread *__osPopThread(OSThread **queue) {
    (void) queue;
    return (OSThread *) 0;
}
OSThread *__osGetActiveQueue(void) {
    return (OSThread *) 0;
}
OSThread *__osGetCurrFaultedThread(void) {
    return (OSThread *) 0;
}
void __osPiGetAccess(void) {
}
void __osPiRelAccess(void) {
}
s32 __osEPiRawStartDma(OSPiHandle *handle, s32 direction, u32 devAddr, void *dramAddr, size_t size) {
    (void) handle;
    (void) direction;
    (void) devAddr;
    (void) dramAddr;
    (void) size;
    return -1; // raw path unsupported; the guest uses osEPiStartDma (os_epi.c)
}
s32 __osMotorAccess(void *pfs, s32 flag) {
    (void) pfs;
    (void) flag;
    return -1;
}
void __osSetHWIntrRoutine(s32 intr, s32 (*handler)(void), void *sp) {
    (void) intr;
    (void) handler;
    (void) sp;
}
void *__osGetHWIntrRoutine(s32 intr) {
    (void) intr;
    return (void *) 0;
}
