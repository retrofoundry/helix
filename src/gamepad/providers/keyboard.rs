use crate::gamepad::providers::{Gamepad, GamepadProvider, GamepadService};
use crate::gamepad::types::{N64Button, OSControllerPad};
use crate::gamepad::utils::MAX_N64_AXIS_RANGE;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct KeyboardGamepadProvider {
    pub keys: Vec<KeyCode>,
}

impl KeyboardGamepadProvider {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }
}

impl Default for KeyboardGamepadProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GamepadProvider for KeyboardGamepadProvider {
    fn scan(&self) -> Vec<Gamepad> {
        let device = Gamepad::new(GamepadService::Keyboard());
        vec![device]
    }

    fn process_events(&mut self) {}

    unsafe fn read(&self, _controllers: &Gamepad, pad: *mut OSControllerPad) {
        unsafe {
            if self.keys.contains(&KeyCode::KeyX) {
                (*pad).button |= N64Button::A as u16;
            }
            if self.keys.contains(&KeyCode::KeyC) {
                (*pad).button |= N64Button::B as u16;
            }
            if self.keys.contains(&KeyCode::KeyZ) {
                (*pad).button |= N64Button::Z as u16;
            }
            if self.keys.contains(&KeyCode::Space) {
                (*pad).button |= N64Button::Start as u16;
            }
            if self.keys.contains(&KeyCode::KeyW) {
                (*pad).stick_y = MAX_N64_AXIS_RANGE as i8;
            }
            if self.keys.contains(&KeyCode::KeyA) {
                (*pad).stick_x = (-MAX_N64_AXIS_RANGE) as i8;
            }
            if self.keys.contains(&KeyCode::KeyS) {
                (*pad).stick_y = (-MAX_N64_AXIS_RANGE) as i8;
            }
            if self.keys.contains(&KeyCode::KeyD) {
                (*pad).stick_x = MAX_N64_AXIS_RANGE as i8;
            }
            if self.keys.contains(&KeyCode::ArrowUp) {
                (*pad).button |= N64Button::CUp as u16;
            }
            if self.keys.contains(&KeyCode::ArrowLeft) {
                (*pad).button |= N64Button::CLeft as u16;
            }
            if self.keys.contains(&KeyCode::ArrowDown) {
                (*pad).button |= N64Button::CDown as u16;
            }
            if self.keys.contains(&KeyCode::ArrowRight) {
                (*pad).button |= N64Button::CRight as u16;
            }
            if self.keys.contains(&KeyCode::KeyT) {
                (*pad).button |= N64Button::DUp as u16;
            }
            if self.keys.contains(&KeyCode::KeyF) {
                (*pad).button |= N64Button::DLeft as u16;
            }
            if self.keys.contains(&KeyCode::KeyG) {
                (*pad).button |= N64Button::DDown as u16;
            }
            if self.keys.contains(&KeyCode::KeyH) {
                (*pad).button |= N64Button::DRight as u16;
            }
            if self.keys.contains(&KeyCode::KeyR) {
                (*pad).button |= N64Button::L as u16;
            }
            if self.keys.contains(&KeyCode::KeyY) {
                (*pad).button |= N64Button::R as u16;
            }
        }
    }

    fn handle_modifiers_changed(&mut self, _modifiers: winit::keyboard::ModifiersState) {}

    fn handle_keyboard_input(&mut self, input: &KeyEvent) {
        let PhysicalKey::Code(key) = input.physical_key else {
            return;
        };

        if input.state == ElementState::Pressed {
            if !self.keys.contains(&key) {
                self.keys.push(key);
            }
        } else {
            self.keys.retain(|&k| k != key);
        }
    }
}
