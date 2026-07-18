#include <PR/os_time.h>
#include <PR/os_misc.h>

// osClockRate lived in the gated-out src/pc/ultra_reimplementation.c — re-home it.
u64 osClockRate = 62500000;

extern u64 HLXGetTime(void);
extern void HLXSetTime(u64 t);
extern u32 HLXGetCount(void);

OSTime osGetTime(void) {
    return HLXGetTime();
}

void osSetTime(OSTime t) {
    HLXSetTime(t);
}

u32 osGetCount(void) {
    return HLXGetCount();
}
