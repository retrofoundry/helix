mod speech_synthesizer;

use std::sync::Mutex;
use lazy_static::lazy_static;
use std::ffi::CStr;
use std::str;

lazy_static! {
    static ref HELIX: Mutex<Helix> = Mutex::new(Helix::new());
}

pub(crate) struct Helix {
    speech_synthesizer: Box<dyn speech_synthesizer::SpeechSynthesizer>,
}

unsafe impl Send for Helix {}

impl Helix {
    pub(crate) fn new() -> Helix {
        Helix {
            speech_synthesizer: Box::new(speech_synthesizer::create()),
        }
    }
}

// C API

#[no_mangle]
pub extern "C" fn SpeechSynthesizerInit() {
    HELIX.lock().unwrap().speech_synthesizer.init();
}

#[no_mangle]
pub extern "C" fn SpeechSynthesizerUninitialize() {
    // HELIX.lock().unwrap().speech_synthesizer.uninitialize();
}

#[no_mangle]
pub extern "C" fn SpeechSynthesizerSpeak(text_raw: *const i8, language_raw: *const i8) {
    let text_str: &CStr = unsafe { CStr::from_ptr(text_raw) };
    let text: &str = str::from_utf8(text_str.to_bytes()).unwrap();

    let language_str: &CStr = unsafe { CStr::from_ptr(language_raw) };
    let language: &str = str::from_utf8(language_str.to_bytes()).unwrap();

    HELIX.lock().unwrap().speech_synthesizer.speak(text, language);
}




// pub fn add(left: usize, right: usize) -> usize {
//     left + right
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
