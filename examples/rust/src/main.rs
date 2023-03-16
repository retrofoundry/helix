use helix::gui::Gui;

fn main() {
    println!("Hello, world!");
    
    // #[cfg(not(target_os = "linux"))]
    // let mut speech_synthesizer = SpeechSynthesizer::new();
    
    // #[cfg(not(target_os = "linux"))] {
    //     speech_synthesizer.init();
    //     speech_synthesizer.set_volume(1.0);
    //     speech_synthesizer.speak("Hello, world!", true);
    // }

    Gui::start();
    
    loop {
        println!("Hello, world!");
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
