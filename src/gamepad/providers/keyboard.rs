use crate::gamepad::providers::{Gamepad, GamepadProvider, GamepadService};
use crate::gamepad::types::{N64Button, OSControllerPad};
use crate::gamepad::utils::MAX_N64_AXIS_RANGE;
use device_query::{DeviceQuery, DeviceState, Keycode};
use log::trace;

pub struct KeyboardGamepadProvider {
    pub api: DeviceState,
}

impl KeyboardGamepadProvider {
    pub fn new() -> Self {
        let api = DeviceState::new();
        Self { api }
    }
}

impl GamepadProvider for KeyboardGamepadProvider {
    fn scan(&self) -> Vec<Gamepad> {
        let device = Gamepad::new(GamepadService::Keyboard());
        return vec![device];
    }

    fn process_events(&mut self) {}

    fn read(&self, controllers: &Gamepad, pad: *mut OSControllerPad) {
        if let GamepadService::Keyboard() = controllers.service {
            // let keys: Vec<Keycode> = device_state.get_keys();
            // println!("Is A pressed? {}", keys.contains(Keycode::A));

            let keys = self.api.get_keys();

            unsafe {
                if keys.contains(&Keycode::X) {
                    (*pad).button |= N64Button::A as u16;
                }
                if keys.contains(&Keycode::C) {
                    (*pad).button |= N64Button::B as u16;
                }
                if keys.contains(&Keycode::Z) {
                    (*pad).button |= N64Button::Z as u16;
                }
                if keys.contains(&Keycode::Space) {
                    (*pad).button |= N64Button::Start as u16;
                }
                if keys.contains(&Keycode::W) {
                    (*pad).stick_y = MAX_N64_AXIS_RANGE as i8;
                }
                if keys.contains(&Keycode::A) {
                    (*pad).stick_x = (-MAX_N64_AXIS_RANGE) as i8;
                }
                if keys.contains(&Keycode::S) {
                    (*pad).stick_y = (-MAX_N64_AXIS_RANGE) as i8;
                }
                if keys.contains(&Keycode::D) {
                    (*pad).stick_x = MAX_N64_AXIS_RANGE as i8;
                }
                if keys.contains(&Keycode::Up) {
                    (*pad).button |= N64Button::CUp as u16;
                }
                if keys.contains(&Keycode::Left) {
                    (*pad).button |= N64Button::CLeft as u16;
                }
                if keys.contains(&Keycode::Down) {
                    (*pad).button |= N64Button::CDown as u16;
                }
                if keys.contains(&Keycode::Right) {
                    (*pad).button |= N64Button::CRight as u16;
                }
                if keys.contains(&Keycode::T) {
                    (*pad).button |= N64Button::DUp as u16;
                }
                if keys.contains(&Keycode::F) {
                    (*pad).button |= N64Button::DLeft as u16;
                }
                if keys.contains(&Keycode::G) {
                    (*pad).button |= N64Button::DDown as u16;
                }
                if keys.contains(&Keycode::H) {
                    (*pad).button |= N64Button::DRight as u16;
                }
            }
        }
    }
}
