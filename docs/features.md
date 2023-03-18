# Features

## Audio
Helix provides functionality for audio playback. Audio playback is simple and Helix provides the following API:

```cpp
/**
 * Creates and sets up the audio player, returning a pointer to the instance or nullptr if creation failed
**/
void* HLXAudioPlayerCreate(uint32_t sampleRate, uint16_t channels);
// Rust: audio_player.init() -> bool

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
