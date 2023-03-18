use helix::gui::Gui;
use std::sync::{Arc, Mutex};

fn main() {
    let show_app_metrics = Arc::new(Mutex::new(false));
    let show_app_metrics_clone = Arc::clone(&show_app_metrics);

    let event_loop = Gui::create_event_loop();
    let gui = Gui::new("Helix Example", &event_loop, |ui| {
        ui.menu("File", || {
            ui.menu_item_config("Quit")
                .shortcut("Ctrl+Q")
                .build();
        });
        ui.separator();
        ui.menu("Edit", || {

        });
    }, move |ui| {
        let mut show_app_metrics = show_app_metrics_clone.lock().unwrap();
        ui.show_metrics_window(&mut show_app_metrics);
    }).unwrap();

    let handler = std::thread::spawn(move || {
        loop {
            println!("Hello, world!");
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    Gui::start(event_loop, gui);
    handler.join().unwrap();
}
