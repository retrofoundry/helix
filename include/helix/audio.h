#ifndef ARIE_LIB_AUDIO_H
#define ARIE_LIB_AUDIO_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void HLXAudioSetup(uint32_t sampleRate, uint16_t channels);
size_t HLXAudioGetBufferredSampleCount();
size_t HLXAudioGetBufferSize();
void HLXAudioPlayBuffer(const uint8_t* buf, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* ARIE_LIB_AUDIO_H */