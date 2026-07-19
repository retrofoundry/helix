#ifndef HELIX_RUNTIME_H
#define HELIX_RUNTIME_H

#include <libultra/ultratypes.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Process lifecycle (helix/src/ultra/mod.rs). */
void HLXRuntimeInit(void);
void HLXRunEventLoop(void);

/* Threads (helix/src/ultra/thread.rs). Guest sp is ignored; t is the OSThread*. */
void HLXThreadCreate(void *t, s32 id, void (*entry)(void *), void *arg, void *sp, s32 pri);
void HLXThreadStart(void *t);
void HLXThreadSetPri(void *t, s32 pri); /* t == NULL means the calling thread */
void HLXThreadStop(void *t);
void HLXThreadYield(void); /* osYieldThread: cooperative reschedule point */

/* Graphics microcode (helix/src/render.rs). The guest declares its GRUCODE build so the
   renderer matches the ROM; values must stay in sync with render.rs's MICROCODE mapping. */
typedef enum {
    HLX_MICROCODE_F3DEX2 = 0,
    HLX_MICROCODE_F3D = 1,
} HLXMicrocode;
void HLXRenderSetMicrocode(u32 microcode);

/* Guest vertex/matrix layout (GBI_FLOATS), orthogonal to the microcode in fast3d. Values must
   stay in sync with render.rs's DATA_FORMAT mapping. */
typedef enum {
    HLX_DATAFMT_FIXED = 0,
    HLX_DATAFMT_FLOAT = 1,
} HLXDataFormat;
void HLXRenderSetDataFormat(u32 format);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_RUNTIME_H */
