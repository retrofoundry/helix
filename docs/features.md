# Features

## Audio
Helix provides functionality for audio playback. Audio playback is simple and Helix provides the following API:

```cpp
/**
 * Creates and sets up the audio player, returning a pointer to the instance or nullptr if creation failed
**/
void* HLXAudioPlayerCreate(uint32_t sampleRate, uint16_t channels);
// Rust: let mut audio_Player = AudiPlayer::new().unwrap();

/**
 * Frees the audio player instance.
**/
void HLXAudioPlayerFree(void* player);
// Rust: no dedicated method, instance drop will deallocate it

/**
 * Returns the amount of data currently in buffer.
**/
int32_t HLXAudioPlayerGetBuffered(void* player);
// Rust: audio_player.buffered() -> i32

/**
 * Returns the amount of data we want the buffer to contain.
**/
int32_t HLXAudioPlayerGetDesiredBuffered(void* player);
// Rust: audio_player.desired_buffer() -> i32

/**
 * Plays the audio from the given buffer - resampling if necessary for audio output device.
**/
void HLXAudioPlayerPlayBuffer(void* player, const uint8_t* buf, size_t len);
// Rust: audio_player.play_buffer(buf: &[u8])
```

## Speech
Helix provides an API for text-to-speech (TTS):

```cpp
/**
 * Creates and sets up the audio synthesizer, returning a pointer to the instance or nullptr if creation failed
**/
void* HLXSpeechSynthesizerCreate(void);
// Rust: let mut speech_synthesizer = SpeechSynthesizer::new().unwrap();

/**
 * Frees the speech synthesizer instance.
**/
void HLXSpeechSynthesizerFree(void* synthesizer);
// Rust: no dedicated method, instance drop will deallocate it

/**
 * Sets the volume for the synthesizer, scale from 0-1.
**/
void HLXSpeechSynthesizerSetVolume(void* synthesizer, float volume);
// Rust: speech_synthesizer.set_volume(volume: f32)

/**
 * Sets the language of the speaker, takes in a en-US type locale.
**/
void HLXSpeechSynthesizerSetLanguage(void* synthesizer, const char* language);
// Rust: speech_synthesizer.set_language(language: &str)

/**
 * Sets the gender of the speaker, accepted values: HLXSpeechSynthesizerGenderFemale/Male/Neutral.
**/
void HLXSpeechSynthesizerSetGender(void* synthesizer, HLXSpeechSynthesizerGender gender);
// Rust: speech_synthesizer.set_gender(gender: HLXSpeechSynthesizerGender)

/**
 * Dictates the given text, specifying whether previous utterances should be interrupted.
**/
void HLXSpeechSynthesizerSpeak(void* synthesizer, const char* text, uint8_t interrupt);
// Rust: speech_synthesizer.speak(text: &str, interrupt: bool)
```
