use anyhow::Result;
use std::ffi::CStr;
use std::str;
use tts::{Gender, LanguageTag, Tts, Voice};

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
    backend: Tts,
    language: LanguageTag<String>,
    gender: SpeechSynthesizerGender,
}

impl SpeechSynthesizer {
    pub fn new() -> Result<Self> {
        let backend = Tts::default()?;

        Ok(Self {
            backend,
            language: LanguageTag::parse("en-US").unwrap().into(),
            gender: SpeechSynthesizerGender::Neutral,
        })
    }

    pub fn set_volume(&mut self, volume: f32) {
        _ = self.backend.set_volume(volume);
    }

    pub fn set_language(&mut self, language: &str) {
        let language = LanguageTag::parse(language);

        match language {
            Ok(language) => {
                self.language = language.into();
                self.set_voice();
            }
            Err(e) => {
                eprintln!("[Speech Synthesizer] Error parsing language: {e}",);
            }
        }
    }

    pub fn set_gender(&mut self, gender: SpeechSynthesizerGender) {
        self.gender = gender;
        self.set_voice();
    }

    pub fn speak(&mut self, text: &str, interrupt: bool) {
        _ = self.backend.speak(text, interrupt);
    }

    fn set_voice(&mut self) {
        // if voices are found, let's try to find one that fits our request
        if let Ok(voices) = self.backend.voices() {
            // filter available voices by language
            let matching_language_voices: Vec<Voice> = voices
                .iter()
                .filter(|v| v.language() == self.language)
                .cloned()
                .collect();

            // filter voices by matching gender
            let matching_gender_voices: Vec<Voice> = matching_language_voices
                .iter()
                .filter(|v| v.gender() == self.gender.to_lib())
                .cloned()
                .collect();

            // if we have a matching voice for both language and gender return it
            if let Some(voice) = matching_gender_voices.first() {
                _ = self.backend.set_voice(voice);
            } else if let Some(voice) = matching_language_voices.first() {
                _ = self.backend.set_voice(voice);
            }
        }
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn SpeechSynthesizerCreate() -> Box<SpeechSynthesizer> {
    match SpeechSynthesizer::new() {
        Ok(synthesizer) => Box::new(synthesizer),
        Err(e) => {
            eprintln!("[Speech Synthesizer] Error creating synthesizer: {e}",);
            unsafe { Box::from_raw(std::ptr::null_mut()) }
        }
    }
}

#[no_mangle]
pub extern "C" fn SpeechSynthesizerFree(synthesizer: Option<Box<SpeechSynthesizer>>) {
    if let Some(synthesizer) = synthesizer {
        drop(synthesizer);
    }
}

#[no_mangle]
pub extern "C" fn SpeechSynthesizerSetVolume(
    synthesizer: Option<&mut SpeechSynthesizer>,
    volume: f32,
) {
    match synthesizer {
        Some(synthesizer) => synthesizer.set_volume(volume),
        None => eprintln!(
            "[Speech Synthesizer] Error setting volume: was given an invalid instance pointer",
        ),
    }
}

#[no_mangle]
pub extern "C" fn SpeechSynthesizerSetLanguage(
    synthesizer: Option<&mut SpeechSynthesizer>,
    language_raw: *const i8,
) {
    let language_str: &CStr = unsafe { CStr::from_ptr(language_raw) };
    let language: &str = str::from_utf8(language_str.to_bytes()).unwrap();

    match synthesizer {
        Some(synthesizer) => synthesizer.set_language(language),
        None => eprintln!(
            "[Speech Synthesizer] Error setting language: was given an invalid instance pointer",
        ),
    }
}

#[no_mangle]
pub extern "C" fn SpeechSynthesizerSetGender(
    synthesizer: Option<&mut SpeechSynthesizer>,
    gender: SpeechSynthesizerGender,
) {
    match synthesizer {
        Some(synthesizer) => synthesizer.set_gender(gender),
        None => eprintln!(
            "[Speech Synthesizer] Error setting gender: was given an invalid instance pointer",
        ),
    }
}

#[no_mangle]
pub extern "C" fn SpeechSynthesizerSpeak(
    synthesizer: Option<&mut SpeechSynthesizer>,
    text_raw: *const i8,
    interrupt: u8,
) {
    let text_str: &CStr = unsafe { CStr::from_ptr(text_raw) };
    let text: &str = str::from_utf8(text_str.to_bytes()).unwrap();

    match synthesizer {
        Some(synthesizer) => synthesizer.speak(text, interrupt != 0),
        None => {
            eprintln!("[Speech Synthesizer] Error speaking: was given an invalid instance pointer",)
        }
    }
}
