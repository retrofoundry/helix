#ifndef HELIX_LIB
#define HELIX_LIB

#include "speech.h"
#include "audio.h"
#include "network.h"
#include "gui.h"

#ifdef __cplusplus
extern "C" {
#endif

bool SpeechFeatureEnabled(void);
bool NetworkFeatureEnabled(void);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB */
