#include <libultra/os_time.h>
#include <libultra/os_misc.h>
#include <libultra/os_message.h>

// osClockRate lived in the gated-out src/pc/ultra_reimplementation.c — re-home it.
u64 osClockRate = 62500000;

extern u64 R64GetTime(void);
extern void R64SetTime(u64 t);
extern u32 R64GetCount(void);

OSTime osGetTime(void) {
    return R64GetTime();
}

void osSetTime(OSTime t) {
    R64SetTime(t);
}

u32 osGetCount(void) {
    return R64GetCount();
}

extern s32 R64SetTimer(OSTimer *timer, OSTime countdown, OSTime interval, OSMesgQueue *mq, OSMesg msg);
extern s32 R64StopTimer(OSTimer *timer);

u32 osSetTimer(OSTimer *timer, OSTime countdown, OSTime interval, OSMesgQueue *mq, OSMesg msg) {
    return R64SetTimer(timer, countdown, interval, mq, msg);
}

u32 osStopTimer(OSTimer *timer) {
    return R64StopTimer(timer);
}
