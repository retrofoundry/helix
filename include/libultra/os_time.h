#ifndef HELIX_LIBULTRA_OS_TIME_H
#define HELIX_LIBULTRA_OS_TIME_H

// libultra timer/clock ABI — helix's canonical, guest-agnostic definition.

#include <libultra/ultratypes.h>
#include <libultra/os_message.h>

typedef u64 OSTime;

typedef struct OSTimer_s {
    struct OSTimer_s *next;
    struct OSTimer_s *prev;
    OSTime interval;
    OSTime value;
    OSMesgQueue *mq;
    OSMesg msg;
} OSTimer;

extern u64 osClockRate;

OSTime osGetTime(void);
void osSetTime(OSTime time);
u32 osSetTimer(OSTimer *timer, OSTime countdown, OSTime interval, OSMesgQueue *mq, OSMesg msg);
u32 osStopTimer(OSTimer *timer);

#endif
