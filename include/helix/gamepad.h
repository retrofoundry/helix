#ifndef HELIX_LIB_CONTROLLER_H
#define HELIX_LIB_CONTROLLER_H

#include <stdint.h>
#include <libultra/os_cont.h>
#include <libultra/ultratypes.h>

#ifdef __cplusplus
extern "C" {
#endif

void* ControllerManagerCreate(void);
s32 ControllerManagerInit(void* manager, u8* gamepad_bits);
void ControllerManagerProcessEvents(void* manager);
void ControllerGetReadData(void* manager, OSContPad* pad);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_CONTROLLER_H */
