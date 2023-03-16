#ifndef HELIX_LIB_GUI_H
#define HELIX_LIB_GUI_H

#ifdef __cplusplus
extern "C" {
#endif

void* HLXGUICreateEventLoop();
void* HLXGUICreate(void* event_loop);
void* HLXGUIStart(void* event_loop, void* gui);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_GUI_H */
