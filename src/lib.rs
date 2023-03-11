mod speech;
mod audio;
mod network;
mod macros;

use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

lazy_static! {
    static ref HELIX: Arc<Mutex<Helix>> = Arc::new(Mutex::new(Helix::new()));
}

pub(crate) struct Helix {
    speech_synthesizer: speech::SpeechSynthesizer,
    audio_player: audio::AudioPlayer,
    tcp_stream: network::TCPStream,
}

impl Helix {
    pub(crate) fn new() -> Helix {
        Helix {
            speech_synthesizer: speech::SpeechSynthesizer::new(),
            audio_player: audio::AudioPlayer::new(),
            tcp_stream: network::TCPStream::new(),
        }
    }
}

unsafe impl Send for Helix {}
unsafe impl Sync for Helix {}
