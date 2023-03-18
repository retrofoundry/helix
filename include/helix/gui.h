#ifndef HELIX_LIB_GUI_H
#define HELIX_LIB_GUI_H

#ifdef __cplusplus
extern "C" {
#endif

void* HLXGUICreateEventLoop();
void* HLXGUICreate(const char* title, void* event_loop, void (*draw_menu_callback)());
void* HLXGUIStart(void* event_loop, void* gui);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_GUI_H */
