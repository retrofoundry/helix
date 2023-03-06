use super::super::speech_synthesizer::SpeechSynthesizer;

#[cfg(target_os = "macos")]
pub struct AVSpeechSynthesizer {}

impl AVSpeechSynthesizer {
    pub fn new() -> Self {
        AVSpeechSynthesizer {}
    }
}

impl SpeechSynthesizer for AVSpeechSynthesizer {
    fn init(&self) {}

    fn uninitialize(&self) {}

    fn speak(&self, text: &str, language: &str) {
        println!("{} {}", text, language);
    }
}