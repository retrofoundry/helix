#ifndef _HELIX_OS_PI_H_
#define _HELIX_OS_PI_H_

#include <stdint.h>
#include <stddef.h>
#include <libultra/ultratypes.h>
#include <libultra/os_message.h>

#define OS_READ  0 // device -> RDRAM
#define OS_WRITE 1 // device <- RDRAM

typedef struct {
    u16 type;
    u8 pri;
    u8 status;
    OSMesgQueue *retQueue;
} OSIoMesgHdr;

typedef struct {
    /*0x00*/ OSIoMesgHdr hdr;
    /*0x08*/ void *dramAddr;
    /*0x0C*/ uintptr_t devAddr;
    /*0x10*/ size_t size;
} OSIoMesg;

s32 osPiStartDma(OSIoMesg *mb, s32 priority, s32 direction, uintptr_t devAddr,
                 void *vAddr, size_t nbytes, OSMesgQueue *mq);

#endif /* _HELIX_OS_PI_H_ */
