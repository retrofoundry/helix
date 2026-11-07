#ifndef REALITY64_INTERNAL
#define REALITY64_INTERNAL

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#include <libultra/os_cont.h>
#include <libultra/ultratypes.h>

#ifdef __cplusplus
extern "C" {
#endif

void reality64Init(void);
bool SpeechFeatureEnabled(void);
bool NetworkFeatureEnabled(void);

// Audio (reality64-owned Arie 0.3 runtime) — src/audio.rs. Fixed-width scalars only; no Arie type
// or header crosses into C. reality64 owns the Arie AudioPlayer + a non-gating in-flight-DMA
// tracker behind one mutex: submit free-pushes and Arie's own software queue is the sole
// backpressure. These are the sole C entry points (called by the os_ai.c shim).
typedef int32_t R64AudioResult;
enum {
    R64_AUDIO_OK = 0,
    R64_AUDIO_UNAVAILABLE = -1,
    R64_AUDIO_INVALID_ARGUMENT = -2,
    R64_AUDIO_QUEUE_FULL = -3,
    R64_AUDIO_BACKEND_ERROR = -4
};
// reality64-internal AI status bits (reality64's own layout, distinct from the guest AI_STATUS_*
// register bits in PR/rcp.h; os_ai.c maps these onto those). FIFO_FULL reflects total host
// occupancy vs the AI target depth; DMA_BUSY means Arie is actively playing.
#define R64_AI_FIFO_FULL 0x1u
#define R64_AI_DMA_BUSY  0x2u
R64AudioResult R64AudioSetSourceRate(uint32_t realized_rate_hz);
R64AudioResult R64AudioSubmit(const void *stereo_i16, uint32_t byte_count);
uint32_t R64AudioCurrentLength(void);
uint32_t R64AudioStatus(void);
R64AudioResult R64AudioClear(void);
void R64AudioTeardown(void);

// GUI
void* GUICreateEventLoop(void);
void* GUICreate(const char* title, void* event_loop, void* gamepad_manager);

// Render (reality64/src/render.rs) — the dedicated render thread owns ultragfx::Renderer and the
// widescreen aspect; this is the sole C-facing getter (repoints R64DisplayGetAspectRatio).
float R64AspectRatio(void);

// Gamepad

void* GamepadManagerCreate(void);
s32 GamepadManagerInit(void* manager, u8* gamepad_bits);
void GamepadManagerProcessEvents(void* manager);
void GamepadManagerGetReadData(void* manager, OSContPad* pad);

// Controller input snapshot (reality64/src/gamepad/snapshot.rs). Runtime path only (thread5):
// the main thread pumps the (!Send) GamepadManager each frame and publishes a plain Send+Sync
// snapshot; thread5 reads it here instead of touching the manager cross-thread. Guarded on
// R64RuntimeActive() in os_cont.c, which falls back to the direct manager path otherwise.
s32  R64ControllerInit(u8* bits);       // report snapshot controller-bits (like osContInit)
void R64ControllerRead(OSContPad* pad); // copy snapshot pad (like osContGetReadData)

// Ultra runtime (Rust core) — message queues, events, PI/DMA
void R64MesgQueueCreate(void* mq, void** msgbuf, s32 count);
s32  R64MesgSend(void* mq, void* msg, s32 flag);
s32  R64MesgRecv(void* mq, void** msg_out, s32 flag);
void R64EventSetMesg(s32 event, void* mq, void* msg);
void R64EventPost(s32 event);
s32  R64PiStartDma(void* mb, s32 dir, size_t devAddr, void* vAddr, size_t nbytes, void* mq);

// R64RomLoad copies and validates a host-endian cart image before guest DMA.
typedef int32_t R64Status;
enum {
    R64_OK = 0,
    R64_ERR_NO_ROM = -1,
    R64_ERR_RANGE = -2,
    R64_ERR_UNSUPPORTED_DEVICE = -3,
    R64_ERR_BAD_ARG = -4,
    R64_ERR_QUEUE = -5,
    R64_ERR_BAD_IMAGE = -6,
    R64_ERR_INTERNAL = -7
};
typedef int32_t R64DevKind;
enum { R64_DEV_CART = 0, R64_DEV_DRIVE = 1, R64_DEV_SRAM = 2, R64_DEV_DEBUG = 3 };
R64Status R64RomLoad(const uint8_t* container, size_t len);
R64Status R64EPiDma(void* mb, R64DevKind kind, uint32_t base, uint32_t devAddr, void* dram,
                    size_t size, s32 dir, void* retQueue);
R64Status R64EPiReadIo(R64DevKind kind, uint32_t base, uint32_t devAddr, uint32_t* out);
bool R64RuntimeActive(void); // true only while the libultra runtime is live (RUNTIME_ACTIVE)

// VI (video interface / retrace clock) — ultra/vi.rs
void R64ViSetModeIndex(u32 index);   // active osViModeTable index (u32::MAX == unset)
u32  R64ViGetTvFamily(u32 osTvType); // shared TV family code (NTSC=0, PAL=1, MPAL=2, unknown=u32::MAX)

// RCP task engine (ultra/rcp.rs)
void R64SpTaskStartGo(void* task);
s32  R64SpTaskYielded(void* task);

// Save (EEPROM) — ultra/save.rs
s32 R64EepromProbe(void);
s32 R64EepromRead(u8 addr, u8* buf, s32 n);
s32 R64EepromWrite(u8 addr, const u8* buf, s32 n);

// Copies SRAM data and posts mb to retQueue. dir: 0 = SRAM->RDRAM, 1 = RDRAM->SRAM.
s32 R64SramDma(void* mb, u32 devAddr, void* dram, size_t size, s32 dir, void* retQueue);

#ifdef __cplusplus
}
#endif

#endif /* REALITY64_INTERNAL */