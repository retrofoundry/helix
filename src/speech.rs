#[cfg(feature = "cpp")]
use std::ffi::CStr;
use std::str;
use tts::*;

#[allow(dead_code)]
#[repr(C)]
pub enum SpeechSynthesizerGender {
    Male,
    Female,
    Neutral,
}

impl SpeechSynthesizerGender {
    fn to_lib(&self) -> Option<Gender> {
        match self {
            SpeechSynthesizerGender::Male => Option::Some(Gender::Male),
            SpeechSynthesizerGender::Female => Option::Some(Gender::Female),
            SpeechSynthesizerGender::Neutral => Option::None,
        }
    }
}

pub struct SpeechSynthesizer {
    config: Option<Config>,
}

pub struct Config {
    backend: Tts,
    language: LanguageTag<String>,
    gender: SpeechSynthesizerGender,
}

impl SpeechSynthesizer {
    pub fn new() -> Self {
        SpeechSynthesizer {
            config: Option::None,
        }
    }

    pub fn init(&mut self) {
        let backend = Tts::default();

        match backend {
            Ok(backend) => {
                self.config = Some(Config {
                    backend,
                    language: LanguageTag::parse("en-US").unwrap().into(),
                    gender: SpeechSynthesizerGender::Neutral,
                })
            }
            Err(e) => {
                eprintln!("[Speech Synthesizer] Error initializing speech synthesizer: {e}");
            }
        }
    }

    pub fn deinit(&mut self) {
        if self.config.is_some() {
            self.config = Option::None;
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        if let Some(config) = self.config.as_mut() {
            _ = config.backend.set_volume(volume);
        }
    }

    pub fn set_language(&mut self, language: &str) {
        if let Some(config) = self.config.as_mut() {
            let language = LanguageTag::parse(language);

            match language {
                Ok(language) => {
                    config.language = language.into();
                    self.set_voice();
                }
                Err(e) => {
                    eprintln!("[Speech Synthesizer] Error parsing language: {e}",);
                }
            }
        }
    }

    pub fn set_gender(&mut self, gender: SpeechSynthesizerGender) {
        if let Some(config) = self.config.as_mut() {
            config.gender = gender;
            self.set_voice();
        }
    }

    pub fn speak(&mut self, text: &str, interrupt: bool) {
        if let Some(config) = self.config.as_mut() {
            _ = config.backend.speak(text, interrupt);
        }
    }

    fn set_voice(&mut self) {
        if let Some(config) = self.config.as_mut() {
            // if voices are found, let's try to find one that fits our request
            if let Ok(voices) = config.backend.voices() {
                // filter available voices by language
                let matching_language_voices: Vec<Voice> = voices
                    .iter()
                    .filter(|v| v.language() == config.language)
                    .cloned()
                    .collect();

                // filter voices by matching gender
                let matching_gender_voices: Vec<Voice> = matching_language_voices
                    .iter()
                    .filter(|v| v.gender() == config.gender.to_lib())
                    .cloned()
                    .collect();

                // if we have a matching voice for both language and gender return it
                if let Some(voice) = matching_gender_voices.first() {
                    _ = config.backend.set_voice(voice);
                } else if let Some(voice) = matching_language_voices.first() {
                    _ = config.backend.set_voice(voice);
                }
            }
        }
    }
}

// MARK: - C API

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerCreate() -> Box<SpeechSynthesizer> {
    Box::new(SpeechSynthesizer::new())
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerInit(synthesizer: Option<&mut SpeechSynthesizer>) {
    synthesizer.unwrap().init();
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerDeinit(synthesizer: Option<Box<SpeechSynthesizer>>) {
    synthesizer.unwrap().deinit();
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerSetVolume(
    synthesizer: Option<&mut SpeechSynthesizer>,
    volume: f32,
) {
    synthesizer.unwrap().set_volume(volume);
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerSetLanguage(
    synthesizer: Option<&mut SpeechSynthesizer>,
    language_raw: *const i8,
) {
    let language_str: &CStr = unsafe { CStr::from_ptr(language_raw) };
    let language: &str = str::from_utf8(language_str.to_bytes()).unwrap();

    synthesizer.unwrap().set_language(language);
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerSetGender(
    synthesizer: Option<&mut SpeechSynthesizer>,
    gender: SpeechSynthesizerGender,
) {
    synthesizer.unwrap().set_gender(gender);
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXSpeechSynthesizerSpeak(
    synthesizer: Option<&mut SpeechSynthesizer>,
    text_raw: *const i8,
    interrupt: u8,
) {
    let text_str: &CStr = unsafe { CStr::from_ptr(text_raw) };
    let text: &str = str::from_utf8(text_str.to_bytes()).unwrap();

    synthesizer.unwrap().speak(text, interrupt != 0);
}
