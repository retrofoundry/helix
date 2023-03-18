# Features

## Audio
Helix provides functionality for audio playback. Audio playback is simple and Helix provides the following API:

```cpp
/**
 * Creates the audio player and returns a pointer to the object
**/
void* HLXAudioPlayerCreate(void);
// Rust: let mut audio_player = AudioPlayer::new();

/**
 * Initializes the audio playe rand sets up output devices
**/
bool HLXAudioPlayerInit(void* player, uint32_t sampleRate, uint16_t channels);
// Rust: audio_player.init() -> bool

/**
 * Deinits the audio player and frees the instance.
**/
void HLXAudioPlayerDeinit(void* player);
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
