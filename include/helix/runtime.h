#ifndef HELIX_RUNTIME_H
#define HELIX_RUNTIME_H

#include <libultra/ultratypes.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Process lifecycle (helix/src/ultra/mod.rs). */
void HLXRuntimeInit(void);
void HLXRunEventLoop(void);
void HLXSetWindowTitle(const char *title); /* from main_func, before the window opens */

/* Threads (helix/src/ultra/thread.rs). Guest sp is ignored; t is the OSThread*. */
void HLXThreadCreate(void *t, s32 id, void (*entry)(void *), void *arg, void *sp, s32 pri);
void HLXThreadStart(void *t);
void HLXThreadSetPri(void *t, s32 pri); /* t == NULL means the calling thread */
void HLXThreadStop(void *t);
void HLXThreadYield(void); /* osYieldThread: cooperative reschedule point */

/* RSP task handlers helix provides. A guest binds each microcode it ships to one of them. */
typedef enum {
    HLX_RSP_GFX_F3D = 1,
    HLX_RSP_GFX_F3DEX2 = 2,
    HLX_RSP_GFX_F3DEX2_NON = 3,
    HLX_RSP_GFX_S2DEX2 = 4,
    HLX_RSP_AUDIO_ACMD = 5,   /* the classic aspMain Acmd command set (cpp/audio/acmd.c) */
    HLX_RSP_JPEG_NJPGDSP = 6, /* Ocarina of Time's njpgdspMain, RGBA5551 output */
    HLX_RSP_SKIP = 7,         /* complete the task without running anything */
} HLXRspHandler;

typedef struct {
    const void *ucode; /* the microcode text address the guest stores in OSTask.t.ucode */
    u32 handler;       /* an HLXRspHandler */
} HLXRspBinding;

/* Binds microcode to handlers. Call once from main_func, before any thread starts. Returns 0, or -1
   for a null or duplicate ucode, an unknown handler, or a second call. */
s32 HLXRspRegister(const HLXRspBinding *bindings, size_t count);

/* Registers live guest memory for display-list reads and write-back until process exit. */
void HLXRegisterGuestMemory(void *start, size_t size);

/* Guest vertex/matrix layout (fixed N64 s16 vs float), orthogonal to the microcode in ultragfx.
   Values must stay in sync with render.rs's DATA_FORMAT mapping. */
typedef enum {
    HLX_DATAFMT_FIXED = 0,
    HLX_DATAFMT_FLOAT = 1,
} HLXDataFormat;
void HLXRenderSetDataFormat(u32 format);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_RUNTIME_H */
