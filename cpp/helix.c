#include <helix/internal.h>

void *_event_loop;
void *_gamepad_manager;
void *_gui;
void *_audio_player;
void *_frame;

// Bridges to setup libultra components
extern void _osContInternalSetup(void* gamepad_manager);

// Helix
void HLXInit() {
    HelixInit();
    _event_loop = GUICreateEventLoop();
    _gamepad_manager = GamepadManagerCreate();
    _osContInternalSetup(_gamepad_manager);
}

// Audio
void HLXAudioSetup(uint32_t sampleRate, uint16_t channels) {
    _audio_player = AudioPlayerCreate(32000, 2);
}

size_t HLXAudioGetBufferredSampleCount() {
    return AudioPlayerGetBufferredSampleCount(_audio_player);
}

size_t HLXAudioGetBufferSize() {
    return AudioPlayerGetBufferSize(_audio_player);
}

void HLXAudioPlayBuffer(const uint8_t* buf, size_t len) {
    AudioPlayerQueueBuffer(_audio_player, buf, len);
}

// Window & Graphics
void HLXDisplaySetup(const char* title, void (*draw_menu)(void*), void (*draw_windows)(void*)) {
    _gui = GUICreate(title, _event_loop, draw_menu, draw_windows, _gamepad_manager); // pass in a possible keyboard observing object
}

void HLXDisplayStartFrame() {
    GUIStartFrame(_gui, _event_loop);
}

void HLXDisplayProcessDrawLists(u64* commands) {
    GUIDrawLists(_gui, commands);
}

void HLXDisplayEndFrame() {
    GUIEndFrame(_gui);
}

f32 HLXDisplayGetAspectRatio() {
    return GUIGetAspectRatio(_gui);
}

void HLXShowProfilerWindow(void* ui, bool* opened) {
    GUIShowProfilerWindow(ui, _gui, opened);
}
