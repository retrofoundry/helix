use super::providers::gilrs::GirlsGamepadProvider;
use super::types::{GamepadBits, OSControllerPad};
use crate::gamepad::providers::{Gamepad, GamepadProvider, GamepadService};
use std::mem::size_of;
use std::os::raw::c_void;
use std::ptr;
use std::ptr::null_mut;

pub struct GamepadManager {
    gamepads: Vec<Gamepad>,
    gamepad_bits: GamepadBits,
    providers: Vec<Box<dyn GamepadProvider>>,
}

impl GamepadManager {
    pub fn new() -> Self {
        Self {
            gamepads: Vec::new(),
            gamepad_bits: null_mut(),
            providers: vec![Box::new(GirlsGamepadProvider::new())],
        }
    }

    pub fn init(&mut self, gamepad_bits: GamepadBits) {
        unsafe {
            *gamepad_bits = 0;
        }

        self.scan_for_controllers();

        unsafe {
            *gamepad_bits = 1;
        }
        self.gamepad_bits = gamepad_bits;
    }

    fn scan_for_controllers(&mut self) {
        self.gamepads.clear();

        for provider in self.providers.iter() {
            for device in provider.scan() {
                self.gamepads.push(device);
            }
        }

        // TODO: Register a keyboard device?
    }

    pub fn write(&mut self, pad: *mut OSControllerPad) {
        // TODO: Handle current slot (*)

        for controller in &self.gamepads {
            match controller.service {
                GamepadService::GilRs(_) => {
                    for provider in &self.providers {
                        provider.read(controller, pad);
                    }
                }
                GamepadService::Keyboard() => {
                    todo!("Implement keyboard gamepad");
                }
            }
        }
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn ControllerManagerCreate() -> Box<GamepadManager> {
    let hub = GamepadManager::new();
    Box::new(hub)
}

#[no_mangle]
extern "C" fn ControllerManagerInit(
    manager: Option<&mut GamepadManager>,
    gamepad_bits: GamepadBits,
) -> i32 {
    let manager = manager.unwrap();
    manager.init(gamepad_bits);

    0
}

#[no_mangle]
extern "C" fn ControllerGetReadData(manager: Option<&mut GamepadManager>, pad: *mut OSControllerPad) {
    unsafe {
        ptr::write_bytes(pad, 0, size_of::<OSControllerPad>());
    }

    let manager = manager.unwrap();
    manager.write(pad);
}
