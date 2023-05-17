#ifndef HELIX_LIB_GUI_H
#define HELIX_LIB_GUI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void* GUICreateEventLoop(void);
void* GUICreate(const char* title, void* event_loop, void (*draw_menu_callback)());
void GUIStartFrame(void* gui, void* event_loop);
void GUIDrawLists(void* gui, void* gfx_context, uint64_t* commands);
void GUIDrawListsDummy(void* gui);
void GUIEndFrame(void* gui);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_GUI_H */
