#ifndef HELIX_LIB_AUDIO_H
#define HELIX_LIB_AUDIO_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void* HLXAudioPlayerCreate(uint32_t sampleRate, uint16_t channels);
void HLXAudioPlayerFree(void* player);

int32_t HLXAudioPlayerGetBuffered(void* player);
int32_t HLXAudioPlayerGetDesiredBuffered(void* player);

void HLXAudioPlayerPlayBuffer(void* player, const uint8_t* buf, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_AUDIO_H */
