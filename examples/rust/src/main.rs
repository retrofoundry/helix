use helix::{gui::Gui, gamepad::manager::GamepadManager};

fn main() {
    helix::init();

    let mut controller_manager = GamepadManager::new();

    let mut value: u8 = 0;
    let controller_bits: *mut u8 = &mut value as *mut u8;
    controller_manager.init(controller_bits);

    let mut gui = Gui::new("Helix Example", |ui| {
        ui.menu("File", || {
            ui.menu_item_config("Quit")
                .shortcut("Ctrl+Q")
                .build();
        });
        ui.separator();
        ui.menu("Edit", || {

        });
    }).unwrap();

    std::thread::spawn(move || {
        loop {
            println!("Hello, world!");
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    loop {
        gui.start_frame();
        gui.draw_lists_dummy();
        gui.end_frame();
    }
}