#ifndef HELIX_LIB
#define HELIX_LIB

#include "speech.h"
#include "audio.h"
#include "network.h"
#include "controller.h"
#include "gui.h"

#ifdef __cplusplus
extern "C" {
#endif

bool HLXSpeechFeatureEnabled(void);
bool HLXNetworkFeatureEnabled(void);
bool HLXControllerFeatureEnabled(void);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB */
