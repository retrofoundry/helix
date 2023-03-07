mod speech;
mod audio;

use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref HELIX: Mutex<Helix> = Mutex::new(Helix::new());
}

pub(crate) struct Helix {
    speech_synthesizer: speech::SpeechSynthesizer,
    audio_player: audio::AudioPlayer,
}

impl Helix {
    pub(crate) fn new() -> Helix {
        Helix {
            speech_synthesizer: speech::SpeechSynthesizer::new(),
            audio_player: audio::AudioPlayer::new(),
        }
    }
}
