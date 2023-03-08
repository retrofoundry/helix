#ifndef HELIX_LIB_AUDIO_H
#define HELIX_LIB_AUDIO_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

bool HLXAudioPlayerInit(void);

int32_t HLXAudioPlayerGetBuffered(void);
int32_t HLXAudioPlayerGetDesiredBuffered(void);

void HLXAudioPlayerPlayBuffer(const uint8_t* buf, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_AUDIO_H */
