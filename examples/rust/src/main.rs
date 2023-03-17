use helix::speech::SpeechSynthesizer;
use helix::controller::hub::ControllerHub;

fn main() {
    println!("Hello, world!");

    let mut control_hub = ControllerHub::new();
    control_hub.init(Box::new(0));
    
    #[cfg(not(target_os = "linux"))]
    let mut speech_synthesizer = SpeechSynthesizer::new();
    
    #[cfg(not(target_os = "linux"))] {
        speech_synthesizer.init();
        speech_synthesizer.set_volume(1.0);
        speech_synthesizer.speak("Hello, world!", true);
    }

    // Wait for the speech to finish.
    std::thread::sleep(std::time::Duration::from_secs(2));
}
