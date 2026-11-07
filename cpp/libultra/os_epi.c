#include <libultra/os_pi.h>
#include <reality64/internal.h>

// The cart base makes ((base | devAddr) & PHYS_MASK) - ROM_BASE recover the VROM offset.
#define PI_CART_DOMAIN1_BASE 0x10000000u
#define DEVICE_TYPE_CART     0
#define DEVICE_TYPE_SRAM     3

static OSPiHandle sCartHandle;

OSPiHandle *osCartRomInit(void) {
    sCartHandle.type = DEVICE_TYPE_CART;
    sCartHandle.baseAddress = PI_CART_DOMAIN1_BASE;
    return &sCartHandle;
}

OSPiHandle *osDriveRomInit(void) {
    return (OSPiHandle *) 0; // no 64DD
}

s32 osEPiStartDma(OSPiHandle *handle, OSIoMesg *mb, s32 direction) {
    if (handle->type == DEVICE_TYPE_SRAM) {
        return R64SramDma((void *) mb, (u32) mb->devAddr, mb->dramAddr, mb->size, direction,
                          (void *) mb->hdr.retQueue);
    }
    return R64EPiDma((void *) mb, R64_DEV_CART, handle->baseAddress, (u32) mb->devAddr,
                     mb->dramAddr, mb->size, direction, (void *) mb->hdr.retQueue);
}

s32 osEPiReadIo(OSPiHandle *handle, u32 devAddr, u32 *data) {
    return R64EPiReadIo(R64_DEV_CART, handle->baseAddress, devAddr, data);
}

s32 osEPiWriteIo(OSPiHandle *handle, u32 devAddr, u32 data) {
    (void) handle;
    (void) devAddr;
    (void) data;
    return -1; // cart ROM is read-only
}
