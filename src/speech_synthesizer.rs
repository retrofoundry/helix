#[cfg(target_os = "macos")]
pub mod av_speech_synthesizer;

#[cfg(target_os = "windows")]
pub mod sapi_speech_synthesizer;

pub trait SpeechSynthesizer {
    fn init(&self);
    fn uninitialize(&self);
    fn speak(&self, text: &str, language: &str);
}

pub fn create() -> impl SpeechSynthesizer {
    #[cfg(target_os = "macos")]
    return av_speech_synthesizer::AVSpeechSynthesizer::new();
    #[cfg(target_os = "windows")]
    return sapi_speech_synthesizer::SAPISpeechSynthesizer::new();
}
