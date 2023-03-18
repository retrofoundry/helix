use helix::gui::Gui;

fn main() {
    let event_loop = Gui::create_event_loop();
    let gui = Gui::new("Helix Example", &event_loop).unwrap();

    let handler = std::thread::spawn(move || {
        loop {
            println!("Hello, world!");
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    Gui::start(event_loop, gui);
    handler.join().unwrap();
}
