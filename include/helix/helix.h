#ifndef HELIX_LIB
#define HELIX_LIB

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// MARK: - Speech Synthesizer

typedef enum {
    HLXSpeechSynthesizerGenderMale,
    HLXSpeechSynthesizerGenderFemale,
    HLXSpeechSynthesizerGenderNeutral
} HLXSpeechSynthesizerGender;

bool HLXSpeechSynthesizerInit(void);
void HLXSpeechSynthesizerDeinit(void);

void HLXSpeechSynthesizerSetVolume(float volume);
void HLXSpeechSynthesizerSetLanguage(const char* language);
void HLXSpeechSynthesizerSetGender(HLXSpeechSynthesizerGender gender);

void HLXSpeechSynthesizerSpeak(const char* text, uint8_t interrupt);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB */
