use super::super::speech_synthesizer::SpeechSynthesizer;

#[cfg(target_os = "windows")]
pub struct SAPISpeechSynthesizer {}

impl SAPISpeechSynthesizer {
    pub fn new() -> Self {
        SAPISpeechSynthesizer {}
    }
}

impl SpeechSynthesizer for SAPISpeechSynthesizer {
    fn init(&self) {
        println!("init");
    }

    fn uninitialize(&self) {
        println!("uninitialize");
    }

    fn speak(&self, text: &str, language: &str) {
        println!("{} {}", text, language);
    }
}