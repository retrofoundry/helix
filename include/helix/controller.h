#ifndef HELIX_LIB_CONTROLLER_H
#define HELIX_LIB_CONTROLLER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void* HLXCreateControllerHub(void);
int32_t HLXControllerInit(void* hub, uint8_t* bits);
void HLXControllerRead(void);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_CONTROLLER_H */
