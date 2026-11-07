#ifndef REALITY64_RUNTIME_H
#define REALITY64_RUNTIME_H

#include <libultra/ultratypes.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Process lifecycle (reality64/src/ultra/mod.rs). */
void R64RuntimeInit(void);
void R64RunEventLoop(void);
void R64SetWindowTitle(const char *title); /* from main_func, before the window opens */

/* Threads (reality64/src/ultra/thread.rs). Guest sp is ignored; t is the OSThread*. */
void R64ThreadCreate(void *t, s32 id, void (*entry)(void *), void *arg, void *sp, s32 pri);
void R64ThreadStart(void *t);
void R64ThreadSetPri(void *t, s32 pri); /* t == NULL means the calling thread */
void R64ThreadStop(void *t);
void R64ThreadYield(void); /* osYieldThread: cooperative reschedule point */

/* RSP task handlers reality64 provides. A guest binds each microcode it ships to one of them. */
typedef enum {
    R64_RSP_GFX_F3D = 1,
    R64_RSP_GFX_F3DEX2 = 2,
    R64_RSP_GFX_F3DEX2_NON = 3,
    R64_RSP_GFX_S2DEX2 = 4,
    R64_RSP_AUDIO_ACMD = 5,   /* the Rust classic US aspMain handler; requires its 704-byte raw data */
    R64_RSP_JPEG_NJPGDSP = 6, /* Ocarina of Time's njpgdspMain, RGBA5551 output */
    R64_RSP_SKIP = 7,         /* complete the task without running anything */
} R64RspHandler;

typedef struct {
    const void *ucode; /* the microcode text address the guest stores in OSTask.t.ucode */
    u32 handler;       /* an R64RspHandler */
} R64RspBinding;

/* Binds microcode to handlers. Call once from main_func, before any thread starts. Returns 0, or -1
   for a null or duplicate ucode, an unknown handler, or a second call. */
s32 R64RspRegister(const R64RspBinding *bindings, size_t count);

/* Registers live guest memory for display-list reads and write-back until process exit. */
void R64RegisterGuestMemory(void *start, size_t size);

/* Guest vertex/matrix layout (fixed N64 s16 vs float), orthogonal to the microcode in ultragfx.
   Values must stay in sync with render.rs's DATA_FORMAT mapping. */
typedef enum {
    R64_DATAFMT_FIXED = 0,
    R64_DATAFMT_FLOAT = 1,
} R64DataFormat;
void R64RenderSetDataFormat(u32 format);

#ifdef __cplusplus
}
#endif

#endif /* REALITY64_RUNTIME_H */
