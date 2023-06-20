# Features

## GUI
Helix provides a GUI library for creating windows and rendering graphics. The GUI library is currently a work in progress and is not yet complete. 

__[ImGui](https://github.com/ocornut/imgui) / [imgui-rs](https://github.com/imgui-rs/imgui-rs) is used for drawing, if you're working on a C/C++ project, you'll need to include the ImGui headers to your project:__

```cpp
// If C++
#include <imgui/imgui.h>
// If C (make sure to set CIMGUI_DEFINE_ENUMS_AND_STRUCTS to 1)
#include <cimgui/cimgui.h>
```

The following API is provided:

```cpp
/**
 * Creates and sets up the window, passes in a function that is to be called to draw the menu bar
**/
void HLXDisplaySetup(const char* title, void (*draw_menu)(), void (*draw_windows)());

/**
 * Used to start a frame and prepare for drawing
**/
void HLXDisplayStartFrame();

/**
 * N64 Gfx commands are passed in to be drawn to the screen
**/
void HLXDisplayProcessDrawLists(u64* commands);

/**
 * Marks the end of the frame
**/
void HLXDisplayEndFrame();

/**
 * Can be used to get the display's aspect ratio
**/
float HLXDisplayGetAspectRatio();
```

## Audio
Helix provides functionality for audio playback. Audio playback is simple and Helix provides the following API:

```cpp
/**
 * Creates and sets up the audio player, returning a pointer to the instance or nullptr if creation failed
**/
void HLXAudioSetup(uint32_t sampleRate, uint16_t channels);

/**
 * Returns the number of samples currently buffered
**/
size_t HLXAudioGetBufferredSampleCount();

/**
 * Returns the size of the available buffer
**/
size_t HLXAudioGetBufferSize();

/**
 * Plays the audio from the given buffer - resampling if necessary for audio output device.
**/
void HLXAudioPlayBuffer(const uint8_t* buf, size_t len);
```

## Speech
Helix provides an API for text-to-speech (TTS):

```cpp
/**
 * Creates and sets up the audio synthesizer, returning a pointer to the instance or nullptr if creation failed
**/
void* SpeechSynthesizerCreate(void);
// Rust: let mut speech_synthesizer = SpeechSynthesizer::new().unwrap();

/**
 * Frees the speech synthesizer instance.
**/
void SpeechSynthesizerFree(void* synthesizer);
// Rust: no dedicated method, instance drop will deallocate it

/**
 * Sets the volume for the synthesizer, scale from 0-1.
**/
void SpeechSynthesizerSetVolume(void* synthesizer, float volume);
// Rust: speech_synthesizer.set_volume(volume: f32)

/**
 * Sets the language of the speaker, takes in a en-US type locale.
**/
void SpeechSynthesizerSetLanguage(void* synthesizer, const char* language);
// Rust: speech_synthesizer.set_language(language: &str)

/**
 * Sets the gender of the speaker, accepted values: SpeechSynthesizerGenderFemale/Male/Neutral.
**/
void SpeechSynthesizerSetGender(void* synthesizer, SpeechSynthesizerGender gender);
// Rust: speech_synthesizer.set_gender(gender: SpeechSynthesizerGender)

/**
 * Dictates the given text, specifying whether previous utterances should be interrupted.
**/
void SpeechSynthesizerSpeak(void* synthesizer, const char* text, uint8_t interrupt);
// Rust: speech_synthesizer.speak(text: &str, interrupt: bool)
```
