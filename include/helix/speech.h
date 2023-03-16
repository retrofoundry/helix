#ifndef HELIX_LIB_SPEECH_H
#define HELIX_LIB_SPEECH_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    HLXSpeechSynthesizerGenderMale,
    HLXSpeechSynthesizerGenderFemale,
    HLXSpeechSynthesizerGenderNeutral
} HLXSpeechSynthesizerGender;

void* HLXSpeechSynthesizerCreate(void);
bool HLXSpeechSynthesizerInit(void* synthesizer);
void HLXSpeechSynthesizerDeinit(void* synthesizer);

void HLXSpeechSynthesizerSetVolume(void* synthesizer, float volume);
void HLXSpeechSynthesizerSetLanguage(void* synthesizer, const char* language);
void HLXSpeechSynthesizerSetGender(void* synthesizer, HLXSpeechSynthesizerGender gender);

void HLXSpeechSynthesizerSpeak(void* synthesizer, const char* text, uint8_t interrupt);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_SPEECH_H */
