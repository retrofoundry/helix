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

#ifdef __cplusplus
}
#endif

#endif /* HELIX_RUNTIME_H */
