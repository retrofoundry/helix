#include <libultra/os_time.h>
#include <libultra/os_misc.h>
#include <libultra/os_message.h>

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

extern s32 HLXSetTimer(OSTimer *timer, OSTime countdown, OSTime interval, OSMesgQueue *mq, OSMesg msg);
extern s32 HLXStopTimer(OSTimer *timer);

u32 osSetTimer(OSTimer *timer, OSTime countdown, OSTime interval, OSMesgQueue *mq, OSMesg msg) {
    return HLXSetTimer(timer, countdown, interval, mq, msg);
}

u32 osStopTimer(OSTimer *timer) {
    return HLXStopTimer(timer);
}
