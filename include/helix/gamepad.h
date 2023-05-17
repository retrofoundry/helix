#ifndef HELIX_LIB_CONTROLLER_H
#define HELIX_LIB_CONTROLLER_H

#include <stdint.h>
#include <libultra/os_cont.h>
#include <libultra/ultratypes.h>

#ifdef __cplusplus
extern "C" {
#endif

void* GamepadManagerCreate(void);
s32 GamepadManagerInit(void* manager, u8* gamepad_bits);
void GamepadManagerProcessEvents(void* manager);
void GamepadManagerGetReadData(void* manager, OSContPad* pad);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_CONTROLLER_H */
