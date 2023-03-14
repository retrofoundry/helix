#[cfg(feature = "audio")]
mod audio;
mod macros;
#[cfg(feature = "network")]
mod network;
#[cfg(feature = "speech")]
mod speech;

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref HELIX: Arc<Mutex<Helix>> = Arc::new(Mutex::new(Helix::new()));
}

pub(crate) struct Helix {
    #[cfg(feature = "speech")]
    speech_synthesizer: speech::SpeechSynthesizer,
    #[cfg(feature = "audio")]
    audio_player: audio::AudioPlayer,
    #[cfg(feature = "network")]
    tcp_stream: network::TCPStream,
}

impl Helix {
    pub(crate) fn new() -> Helix {
        Helix {
            #[cfg(feature = "speech")]
            speech_synthesizer: speech::SpeechSynthesizer::new(),
            #[cfg(feature = "audio")]
            audio_player: audio::AudioPlayer::new(),
            #[cfg(feature = "network")]
            tcp_stream: network::TCPStream::new(),
        }
    }
}

unsafe impl Send for Helix {}
unsafe impl Sync for Helix {}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HLXAudioFeatureEnabled() -> bool {
    #[cfg(feature = "audio")]
    return true;
    #[cfg(not(feature = "audio"))]
    return false;
}

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
