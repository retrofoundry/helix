use crate::gamepad::providers::{Gamepad, GamepadProvider, GamepadService};
use crate::gamepad::types::{N64Button, OSControllerPad};
use crate::gamepad::utils::MAX_N64_AXIS_RANGE;
use winit::event::VirtualKeyCode;

pub struct KeyboardGamepadProvider {
    pub keys: Vec<VirtualKeyCode>,
}

impl KeyboardGamepadProvider {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }
}

impl GamepadProvider for KeyboardGamepadProvider {
    fn scan(&self) -> Vec<Gamepad> {
        let device = Gamepad::new(GamepadService::Keyboard());
        vec![device]
    }

    fn process_events(&mut self) {}

    fn read(&self, _controllers: &Gamepad, pad: *mut OSControllerPad) {
        unsafe {
            if self.keys.contains(&VirtualKeyCode::X) {
                (*pad).button |= N64Button::A as u16;
            }
            if self.keys.contains(&VirtualKeyCode::C) {
                (*pad).button |= N64Button::B as u16;
            }
            if self.keys.contains(&VirtualKeyCode::Z) {
                (*pad).button |= N64Button::Z as u16;
            }
            if self.keys.contains(&VirtualKeyCode::Space) {
                (*pad).button |= N64Button::Start as u16;
            }
            if self.keys.contains(&VirtualKeyCode::W) {
                (*pad).stick_y = MAX_N64_AXIS_RANGE as i8;
            }
            if self.keys.contains(&VirtualKeyCode::A) {
                (*pad).stick_x = (-MAX_N64_AXIS_RANGE) as i8;
            }
            if self.keys.contains(&VirtualKeyCode::S) {
                (*pad).stick_y = (-MAX_N64_AXIS_RANGE) as i8;
            }
            if self.keys.contains(&VirtualKeyCode::D) {
                (*pad).stick_x = MAX_N64_AXIS_RANGE as i8;
            }
            if self.keys.contains(&VirtualKeyCode::Up) {
                (*pad).button |= N64Button::CUp as u16;
            }
            if self.keys.contains(&VirtualKeyCode::Left) {
                (*pad).button |= N64Button::CLeft as u16;
            }
            if self.keys.contains(&VirtualKeyCode::Down) {
                (*pad).button |= N64Button::CDown as u16;
            }
            if self.keys.contains(&VirtualKeyCode::Right) {
                (*pad).button |= N64Button::CRight as u16;
            }
            if self.keys.contains(&VirtualKeyCode::T) {
                (*pad).button |= N64Button::DUp as u16;
            }
            if self.keys.contains(&VirtualKeyCode::F) {
                (*pad).button |= N64Button::DLeft as u16;
            }
            if self.keys.contains(&VirtualKeyCode::G) {
                (*pad).button |= N64Button::DDown as u16;
            }
            if self.keys.contains(&VirtualKeyCode::H) {
                (*pad).button |= N64Button::DRight as u16;
            }
            if self.keys.contains(&VirtualKeyCode::R) {
                (*pad).button |= N64Button::L as u16;
            }
            if self.keys.contains(&VirtualKeyCode::Y) {
                (*pad).button |= N64Button::R as u16;
            }
        }
    }

    fn handle_modifiers_changed(&mut self, _modifiers: winit::event::ModifiersState) {}

    fn handle_keyboard_input(&mut self, input: winit::event::KeyboardInput) {
        if input.state == winit::event::ElementState::Pressed {
            if let Some(key) = input.virtual_keycode {
                self.keys.push(key);
            }
        } else if let Some(key) = input.virtual_keycode {
            self.keys.retain(|&k| k != key);
        }
    }
}
