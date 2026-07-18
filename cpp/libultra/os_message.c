#include <libultra/os_message.h>
#include <helix/internal.h>

void osCreateMesgQueue(OSMesgQueue *mq, OSMesg *msgBuf, s32 count) {
    // Keep the N64 struct fields coherent for any game code that peeks.
    mq->validCount = 0;
    mq->first = 0;
    mq->msgCount = count;
    mq->msg = msgBuf;
    HLXMesgQueueCreate((void *) mq, (void **) msgBuf, count);
}

s32 osSendMesg(OSMesgQueue *mq, OSMesg msg, s32 flag) {
    return HLXMesgSend((void *) mq, (void *) msg, flag);
}

s32 osJamMesg(OSMesgQueue *mq, OSMesg msg, s32 flag) {
    // sm64-US never calls osJamMesg; tail-insert.
    return HLXMesgSend((void *) mq, (void *) msg, flag);
}

s32 osRecvMesg(OSMesgQueue *mq, OSMesg *msg, s32 flag) {
    return HLXMesgRecv((void *) mq, (void **) msg, flag);
}

void osSetEventMesg(OSEvent e, OSMesgQueue *mq, OSMesg msg) {
    HLXEventSetMesg((s32) e, (void *) mq, (void *) msg);
}
