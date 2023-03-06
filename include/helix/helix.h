#ifndef HELIX_LIB
#define HELIX_LIB

#ifdef __cplusplus
extern "C" {
#endif

// MARK: - Speech Synthesizer

bool HLXSpeechSynthesizerInit(void);
void HLXSpeechSynthesizerUninitialize(void);
void HLXSpeechSynthesizerSpeak(const char* text, const char* language);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB */
