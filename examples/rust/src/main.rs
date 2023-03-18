use helix::speech::SpeechSynthesizer;

fn main() {
    println!("Hello, world!");
    
    #[cfg(not(target_os = "linux"))]
    let mut speech_synthesizer = SpeechSynthesizer::new().unwrap();
    
    #[cfg(not(target_os = "linux"))] {
        speech_synthesizer.set_volume(1.0);
        speech_synthesizer.speak("Hello, world!", true);
    }

    // Wait for the speech to finish.
    std::thread::sleep(std::time::Duration::from_secs(2));
}
