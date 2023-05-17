use crate::gamepad::providers::{Gamepad, GamepadProvider, GamepadService};
use crate::gamepad::types::{N64Button, OSControllerPad};
use crate::gamepad::utils::map_stick_value_to_n64;
use gilrs::{Axis, Button, Gilrs};
use log::debug;

pub struct GirlsGamepadProvider {
    pub api: Gilrs,
}

impl GirlsGamepadProvider {
    pub fn new() -> Self {
        let api = Gilrs::new().unwrap();
        Self { api }
    }
}

impl GamepadProvider for GirlsGamepadProvider {
    fn scan(&self) -> Vec<Gamepad> {
        let mut devices: Vec<Gamepad> = Vec::new();

        for (id, gamepad) in self.api.gamepads() {
            debug!("Found gamepad: {}", gamepad.name());
            devices.push(Gamepad::new(GamepadService::GilRs(id)));
        }

        devices
    }

    fn process_events(&mut self) {
        while let Some(_ev) = self.api.next_event() {}
    }

    fn read(&self, controller: &Gamepad, pad: *mut OSControllerPad) {
        if let GamepadService::GilRs(gamepad_id) = controller.service {
            let gamepad = self.api.gamepad(gamepad_id);

            if !gamepad.is_connected() {
                debug!("Gamepad is not connected");
                return;
            }

            unsafe {
                if gamepad.is_pressed(Button::Start) {
                    (*pad).button |= N64Button::Start as u16;
                }
                if gamepad.is_pressed(Button::LeftTrigger) {
                    (*pad).button |= N64Button::L as u16;
                }
                if gamepad.is_pressed(Button::RightTrigger2) {
                    (*pad).button |= N64Button::R as u16;
                }
                if gamepad.is_pressed(Button::LeftTrigger2) {
                    (*pad).button |= N64Button::Z as u16;
                }
                if gamepad.is_pressed(Button::RightTrigger) {
                    (*pad).button |= N64Button::CRight as u16;
                }
                if gamepad.is_pressed(Button::North) {
                    (*pad).button |= N64Button::CLeft as u16;
                }
                if gamepad.is_pressed(Button::South) {
                    (*pad).button |= N64Button::A as u16;
                }
                if gamepad.is_pressed(Button::West) {
                    (*pad).button |= N64Button::B as u16;
                }
                if gamepad.is_pressed(Button::East) {
                    (*pad).button |= N64Button::CDown as u16;
                }
                if gamepad.is_pressed(Button::DPadUp) {
                    (*pad).button |= N64Button::DUp as u16;
                }
                if gamepad.is_pressed(Button::DPadDown) {
                    (*pad).button |= N64Button::DDown as u16;
                }
                if gamepad.is_pressed(Button::DPadLeft) {
                    (*pad).button |= N64Button::DLeft as u16;
                }
                if gamepad.is_pressed(Button::DPadRight) {
                    (*pad).button |= N64Button::DRight as u16;
                }

                let left_x = gamepad.value(Axis::LeftStickX);
                let left_y = gamepad.value(Axis::LeftStickY);
                let _right_x = gamepad.value(Axis::RightStickX);
                let right_y = gamepad.value(Axis::RightStickY);

                if right_y > 0.3 {
                    (*pad).button |= N64Button::CUp as u16;
                }

                if let Some((adjusted_x, adjusted_y)) = map_stick_value_to_n64(left_x, left_y, 1.0)
                {
                    (*pad).stick_x = adjusted_x;
                    (*pad).stick_y = adjusted_y;
                }
            }
        }
    }
}
