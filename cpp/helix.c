#include <helix/internal.h>

static void *_event_loop;
static void *_gui;
static void *_audio_player;
static void *_frame;

void HLXInit() {
    HelixInit();
    _event_loop = GUICreateEventLoop();
}

// Audio
void HLXAudioSetup(uint32_t sampleRate, uint16_t channels) {
    _audio_player = AudioPlayerCreate(32000, 2);
}

size_t HLXAudioGetBufferredSampleCount() {
    AudioPlayerGetBufferredSampleCount(_audio_player);
}

size_t HLXAudioGetBufferSize() {
    AudioPlayerGetBufferSize(_audio_player);
}

void HLXAudioPlayBuffer(const uint8_t* buf, size_t len) {
    AudioPlayerQueueBuffer(_audio_player, buf, len);
}

// Window & Graphics
void HLXDisplaySetup(const char* title, void (*draw_menu)(void*), void (*draw_windows)(void*)) {
    _gui = GUICreate(title, _event_loop, draw_menu, draw_windows);
}

void HLXDisplayStartFrame() {
    _frame = GUIStartFrame(_gui, _event_loop);
}

void HLXDisplayProcessDrawLists(u64* commands) {
    GUIDrawLists(_gui, _frame, commands);
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
