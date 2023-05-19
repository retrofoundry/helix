use helix::{gui::Gui, gamepad::manager::GamepadManager, fast3d::graphics::{dummy_device::DummyGraphicsDevice, GraphicsContext}};

fn main() {
    helix::init();

    // Gamepad initialization

    let mut gamepad_manager = GamepadManager::new();

    let mut value: u8 = 0;
    let controller_bits: *mut u8 = &mut value as *mut u8;
    gamepad_manager.init(controller_bits);

    // Gui initialization

    let mut event_loop_wrapper = Gui::create_event_loop();

    let mut gui = Gui::new("Helix Example", &event_loop_wrapper, |ui| {
        ui.menu("File", || {
            ui.menu_item_config("Quit")
                .shortcut("Ctrl+Q")
                .build();
        });
        ui.separator();
        ui.menu("Edit", || {

        });
    }).unwrap();

    loop {
        gui.start_frame(&mut event_loop_wrapper);
        gui.draw_lists_dummy();
        gui.end_frame();
    }
}