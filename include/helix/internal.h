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

// Audio (helix-owned Arie 0.3 runtime) — src/audio.rs. Fixed-width scalars only; no Arie type
// or header crosses into C. Helix owns the Arie AudioPlayer + a non-gating in-flight-DMA
// tracker behind one mutex: submit free-pushes and Arie's own software queue is the sole
// backpressure. These are the sole C entry points (called by the os_ai.c shim).
typedef int32_t HLXAudioResult;
enum {
    HLX_AUDIO_OK = 0,
    HLX_AUDIO_UNAVAILABLE = -1,
    HLX_AUDIO_INVALID_ARGUMENT = -2,
    HLX_AUDIO_QUEUE_FULL = -3,
    HLX_AUDIO_BACKEND_ERROR = -4
};
// Helix-internal AI status bits (Helix's own layout, distinct from the guest AI_STATUS_*
// register bits in PR/rcp.h; os_ai.c maps these onto those). FIFO_FULL reflects total host
// occupancy vs the AI target depth; DMA_BUSY means Arie is actively playing.
#define HLX_AI_FIFO_FULL 0x1u
#define HLX_AI_DMA_BUSY  0x2u
HLXAudioResult HLXAudioSetSourceRate(uint32_t realized_rate_hz);
HLXAudioResult HLXAudioSubmit(const void *stereo_i16, uint32_t byte_count);
uint32_t HLXAudioCurrentLength(void);
uint32_t HLXAudioStatus(void);
HLXAudioResult HLXAudioClear(void);
void HLXAudioTeardown(void);

// GUI
void* GUICreateEventLoop(void);
void* GUICreate(const char* title, void* event_loop, void* gamepad_manager);

// Render (helix/src/render.rs) — the dedicated render thread owns fast3d::Renderer and the
// widescreen aspect; this is the sole C-facing getter (repoints HLXDisplayGetAspectRatio).
float HLXAspectRatio(void);

// Gamepad

void* GamepadManagerCreate(void);
s32 GamepadManagerInit(void* manager, u8* gamepad_bits);
void GamepadManagerProcessEvents(void* manager);
void GamepadManagerGetReadData(void* manager, OSContPad* pad);

// Controller input snapshot (helix/src/gamepad/snapshot.rs). Runtime path only (thread5):
// the main thread pumps the (!Send) GamepadManager each frame and publishes a plain Send+Sync
// snapshot; thread5 reads it here instead of touching the manager cross-thread. Guarded on
// HLXRuntimeActive() in os_cont.c, which falls back to the direct manager path otherwise.
s32  HLXControllerInit(u8* bits);       // report snapshot controller-bits (like osContInit)
void HLXControllerRead(OSContPad* pad); // copy snapshot pad (like osContGetReadData)

// Ultra runtime (Rust core) — message queues, events, PI/DMA
void HLXMesgQueueCreate(void* mq, void** msgbuf, s32 count);
s32  HLXMesgSend(void* mq, void* msg, s32 flag);
s32  HLXMesgRecv(void* mq, void** msg_out, s32 flag);
void HLXEventSetMesg(s32 event, void* mq, void* msg);
void HLXEventPost(s32 event);
s32  HLXPiStartDma(void* mb, s32 dir, size_t devAddr, void* vAddr, size_t nbytes, void* mq);
bool HLXRuntimeActive(void); // true only while the libultra runtime is live (RUNTIME_ACTIVE)

// VI (video interface / retrace clock) — ultra/vi.rs
void HLXViSetModeIndex(u32 index);   // active osViModeTable index (u32::MAX == unset)
u32  HLXViGetTvFamily(u32 osTvType); // shared TV family code (NTSC=0, PAL=1, MPAL=2, unknown=u32::MAX)

// RCP task engine (ultra/rcp.rs)
void HLXSpTaskStartGo(void* task);
s32  HLXSpTaskYielded(void* task);

// Save (EEPROM) — ultra/save.rs
s32 HLXEepromProbe(void);
s32 HLXEepromRead(u8 addr, u8* buf, s32 n);
s32 HLXEepromWrite(u8 addr, const u8* buf, s32 n);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_INTERNAL */