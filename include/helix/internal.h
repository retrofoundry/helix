#ifndef HELIX_INTERNAL
#define HELIX_INTERNAL

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#include <libultra/os_cont.h>
#include <libultra/ultratypes.h>

#ifdef __cplusplus
extern "C" {
#endif

void HelixInit(void);
bool SpeechFeatureEnabled(void);
bool NetworkFeatureEnabled(void);

// AudioPlayer (via Arie)
void* AudioPlayerCreate(uint32_t sampleRate, uint16_t channels);
void AudioPlayerFree(void* player);

size_t AudioPlayerGetBufferredSampleCount(void* player);
size_t AudioPlayerGetBufferSize(void* player);

void AudioPlayerPlay(void* player);
void AudioPlayerPause(void* player);

void AudioPlayerQueueBuffer(void* player, const uint8_t* buf, size_t len);

// GUI
void* GUICreateEventLoop(void);
void* GUICreate(const char* title, void* event_loop, void (*draw_menu_callback)());
void* GUICreateGraphicsContext(void* gui);
void GUIStartFrame(void* gui, void* event_loop);
void GUIDrawLists(void* gui, void* gfx_context, uint64_t* commands);
void GUIEndFrame(void* gui);

f32 GUIGetAspectRatio(void* gui);

// Gamepad

void* GamepadManagerCreate(void);
s32 GamepadManagerInit(void* manager, u8* gamepad_bits);
void GamepadManagerProcessEvents(void* manager);
void GamepadManagerGetReadData(void* manager, OSContPad* pad);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_INTERNAL */