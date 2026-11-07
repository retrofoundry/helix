#ifndef _REALITY64_OS_PI_H_
#define _REALITY64_OS_PI_H_

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
    /*0x10*/ void *dramAddr;
    // 32-bit as on the console: the guest's own OSIoMesg declares `u32 devAddr` and a 4-byte size,
    // and reality64 reads the guest's struct, so a wider field picks up the neighbouring bytes.
    /*0x18*/ uint32_t devAddr;
    /*0x1C*/ uint32_t size;
} OSIoMesg;

// Only reality64 reads this handle (the guest holds it opaquely), so it needn't match the guest's layout.
typedef struct OSPiHandle {
    struct OSPiHandle *next;
    u8 type;
    u8 latency, pageSize, relDuration, pulse, domain;
    u32 baseAddress;
    u32 speed;
} OSPiHandle;

s32 osPiStartDma(OSIoMesg *mb, s32 priority, s32 direction, uintptr_t devAddr,
                 void *vAddr, size_t nbytes, OSMesgQueue *mq);

OSPiHandle *osCartRomInit(void);
OSPiHandle *osDriveRomInit(void);
s32 osEPiStartDma(OSPiHandle *handle, OSIoMesg *mb, s32 direction);
s32 osEPiReadIo(OSPiHandle *handle, u32 devAddr, u32 *data);
s32 osEPiWriteIo(OSPiHandle *handle, u32 devAddr, u32 data);

#endif /* _REALITY64_OS_PI_H_ */
