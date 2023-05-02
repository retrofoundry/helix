#ifndef HELIX_LIB_SPEECH_H
#define HELIX_LIB_SPEECH_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    SpeechSynthesizerGenderMale,
    SpeechSynthesizerGenderFemale,
    SpeechSynthesizerGenderNeutral
} SpeechSynthesizerGender;

void* SpeechSynthesizerCreate(void);
void SpeechSynthesizerFree(void* synthesizer);

void SpeechSynthesizerSetVolume(void* synthesizer, float volume);
void SpeechSynthesizerSetLanguage(void* synthesizer, const char* language);
void SpeechSynthesizerSetGender(void* synthesizer, SpeechSynthesizerGender gender);

void SpeechSynthesizerSpeak(void* synthesizer, const char* text, uint8_t interrupt);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_SPEECH_H */
