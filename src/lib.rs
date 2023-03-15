mod audio;
mod macros;
#[cfg(feature = "network")]
mod network;
#[cfg(feature = "speech")]
mod speech;

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref AUDIO_PLAYER : Arc<Mutex<audio::AudioPlayer>> = Arc::new(Mutex::new(audio::AudioPlayer::new()));
    #[cfg(feature = "speech")]
    static ref SPEECH_SYNTHESIZER : Arc<Mutex<speech::SpeechSynthesizer>> = Arc::new(Mutex::new(speech::SpeechSynthesizer::new()));
    #[cfg(feature = "network")]
    static ref TCP_STREAM : Arc<Mutex<network::TCPStream>> = Arc::new(Mutex::new(network::TCPStream::new()));
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HLXSpeechFeatureEnabled() -> bool {
    #[cfg(feature = "speech")]
    return true;
    #[cfg(not(feature = "speech"))]
    return false;
}

#[no_mangle]
pub extern "C" fn HLXNetworkFeatureEnabled() -> bool {
    #[cfg(feature = "network")]
    return true;
    #[cfg(not(feature = "network"))]
    return false;
}
