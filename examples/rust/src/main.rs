use helix::gui::Gui;

fn main() {
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
        gui.draw_lists();
        gui.end_frame();
    }
}