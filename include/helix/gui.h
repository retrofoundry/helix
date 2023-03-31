#ifndef HELIX_LIB_GUI_H
#define HELIX_LIB_GUI_H

#ifdef __cplusplus
extern "C" {
#endif

void* GUICreate(const char* title, void (*draw_menu_callback)());
void* GUIStartFrame(void* gui);
void* GUIDrawLists(void* gui);
void* GUIEndFrame(void* gui);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_GUI_H */
