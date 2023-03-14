#ifndef HELIX_LIB
#define HELIX_LIB

#include "speech.h"
#include "audio.h"
#include "network.h"

#ifdef __cplusplus
extern "C" {
#endif

bool HLXSupportsAudio(void);
bool HLXSupportsSpeech(void);
bool HLXSupportsNetwork(void);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB */
