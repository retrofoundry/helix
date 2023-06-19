#ifndef HELIX_LIB_GUI_H
#define HELIX_LIB_GUI_H

#include <stdint.h>

#include <libultra/ultratypes.h>

#ifdef __cplusplus
extern "C" {
#endif

void HLXDisplaySetup(const char* title, void (*draw_menu)());
void HLXDisplayStartFrame();
void HLXDisplayProcessDrawLists(u64* commands);
void HLXDisplayEndFrame();
float HLXDisplayGetAspectRatio();

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_GUI_H */
