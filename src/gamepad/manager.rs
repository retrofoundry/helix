use super::providers::gilrs::GirlsGamepadProvider;
use super::types::{GamepadBits, OSControllerPad};
use crate::gamepad::providers::keyboard::KeyboardGamepadProvider;
use crate::gamepad::providers::{Gamepad, GamepadProvider};

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
            providers: vec![
                Box::new(GirlsGamepadProvider::new()),
                Box::new(KeyboardGamepadProvider::new()),
            ],
        }
    }

    pub fn init(&mut self, gamepad_bits: GamepadBits) {
        unsafe {
            *gamepad_bits = 0;
        }

        self.gamepad_bits = gamepad_bits;
        self.scan_for_controllers();
    }

    pub fn process_events(&mut self) {
        // TODO: Perhaps update controllers at a slower rate than the game loop?
        self.scan_for_controllers();

        for provider in self.providers.iter_mut() {
            provider.process_events();
        }
    }

    pub fn read(&mut self, pad: *mut OSControllerPad) {
        // TODO: Handle current slot?

        unsafe {
            (*pad).button = 0;
            (*pad).stick_x = 0;
            (*pad).stick_y = 0;
            (*pad).errno = 0;
        }

        for controller in &self.gamepads {
            for provider in &self.providers {
                provider.read(controller, pad);
            }
        }
    }

    fn scan_for_controllers(&mut self) {
        self.gamepads.clear();

        for provider in &self.providers {
            for device in provider.scan() {
                self.gamepads.push(device);
            }
        }

        unsafe {
            *self.gamepad_bits = if !self.gamepads.is_empty() { 1 } else { 0 };
        }
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn GamepadManagerCreate() -> Box<GamepadManager> {
    let hub = GamepadManager::new();
    Box::new(hub)
}

#[no_mangle]
extern "C" fn GamepadManagerInit(
    manager: Option<&mut GamepadManager>,
    gamepad_bits: GamepadBits,
) -> i32 {
    let manager = manager.unwrap();
    manager.init(gamepad_bits);

    0
}

#[no_mangle]
extern "C" fn GamepadManagerProcessEvents(manager: Option<&mut GamepadManager>) {
    let manager = manager.unwrap();
    manager.process_events();
}

#[no_mangle]
extern "C" fn GamepadManagerGetReadData(
    manager: Option<&mut GamepadManager>,
    pad: *mut OSControllerPad,
) {
    let manager = manager.unwrap();
    manager.read(pad);
}
