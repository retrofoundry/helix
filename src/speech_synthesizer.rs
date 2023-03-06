use tts::*;

use crate::HELIX;
use std::ffi::CStr;
use std::str;

pub struct SpeechSynthesizer {
    backend: Option<Tts>,
}

impl SpeechSynthesizer {
    pub fn new() -> Self {
        SpeechSynthesizer {
            backend: Option::None,
        }
    }

    pub fn init(&mut self) {
        let backend = Tts::default();

        match backend {
            Ok(backend) => {
                self.backend = Option::Some(backend);
            }
            Err(e) => {
                println!("Error initializing speech synthesizer: {}", e);
            }
        }
    }

    pub fn uninitialize(&mut self) {
        if let Some(_) = &self.backend {
            self.backend = Option::None;
        }
    }

    pub fn speak(&mut self, text: &str, language: &str) {
        // if the backend is initialized, let's try to speak
        if let Some(backend) = self.backend.as_mut() {
            // if voices are found, let's try to find one for the given language
            if let Ok(voices) = backend.voices() {
                let voice = &voices
                    .iter()
                    .find(|v| v.language().starts_with(language))
                    .unwrap();

                _ = backend.set_voice(voice);
            }
            
            _ = backend.speak(text, true);
        }
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerInit() {
    HELIX.lock().unwrap().speech_synthesizer.init();
}

#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerUninitialize() {
    HELIX.lock().unwrap().speech_synthesizer.uninitialize();
}

#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerSpeak(text_raw: *const i8, language_raw: *const i8) {
    let text_str: &CStr = unsafe { CStr::from_ptr(text_raw) };
    let text: &str = str::from_utf8(text_str.to_bytes()).unwrap();

    let language_str: &CStr = unsafe { CStr::from_ptr(language_raw) };
    let language: &str = str::from_utf8(language_str.to_bytes()).unwrap();

    HELIX.lock().unwrap().speech_synthesizer.speak(text, language);
}