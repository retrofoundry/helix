use crate::gamepad::providers::{Gamepad, GamepadProvider, GamepadService};
use crate::gamepad::types::{N64Button, OSControllerPad};
use gilrs::{Axis, Button, Gilrs};
use log::{debug, info, trace};
use std::sync::{Arc, Mutex};

pub struct GirlsGamepadProvider {
    pub api: Gilrs,
}

impl GirlsGamepadProvider {
    pub fn new() -> Self {
        let api = Gilrs::new().unwrap();
        trace!("Connected gamepads: {}", api.gamepads().count());
        Self { api }
    }
}

impl GamepadProvider for GirlsGamepadProvider {
    fn scan(&self) -> Vec<Gamepad> {
        trace!("Scanning for gamepads...");
        let mut devices: Vec<Gamepad> = Vec::new();

        for (id, gamepad) in self.api.gamepads() {
            debug!("Found gamepad: {}", gamepad.name());
            devices.push(Gamepad::new(GamepadService::GilRs(id)));
        }

        devices
    }

    fn read(&self, controller: &Gamepad, pad: *mut OSControllerPad) {
        if let GamepadService::GilRs(gamepad_id) = controller.service {
            let gamepad = self.api.gamepad(gamepad_id);
            // TODO: should we unlock the api right away?

            if !gamepad.is_connected() {
                return;
            }

            // if (gp->wButtons & XINPUT_GAMEPAD_START) pad->button |= START_BUTTON;
            // if (gp->wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER) pad->button |= Z_TRIG;
            // if (gp->bLeftTrigger > XINPUT_GAMEPAD_TRIGGER_THRESHOLD) pad->button |= Z_TRIG;
            // if (gp->wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER) pad->button |= R_TRIG;
            // if (gp->bRightTrigger > XINPUT_GAMEPAD_TRIGGER_THRESHOLD) pad->button |= R_TRIG;
            // if (gp->wButtons & XINPUT_GAMEPAD_A) pad->button |= A_BUTTON;
            // if (gp->wButtons & XINPUT_GAMEPAD_X) pad->button |= B_BUTTON;
            // if (gp->wButtons & XINPUT_GAMEPAD_DPAD_LEFT) pad->button |= L_TRIG;
            // if (gp->sThumbRX < -0x4000) pad->button |= L_CBUTTONS;
            // if (gp->sThumbRX > 0x4000) pad->button |= R_CBUTTONS;
            // if (gp->sThumbRY < -0x4000) pad->button |= D_CBUTTONS;
            // if (gp->sThumbRY > 0x4000) pad->button |= U_CBUTTONS;

            // if (buttons & 0x0001) pad->button |= START_BUTTON;
            // if (buttons & 0x0008) pad->button |= Z_TRIG;
            // if (buttons & 0x0004) pad->button |= R_TRIG;
            // if (buttons & 0x0100) pad->button |= A_BUTTON;
            // if (buttons & 0x0200) pad->button |= B_BUTTON;
            // if (buttons & 0x1000) pad->button |= L_TRIG;
            // if (axis[2] < 0x40) pad->button |= L_CBUTTONS;
            // if (axis[2] > 0xC0) pad->button |= R_CBUTTONS;
            // if (axis[3] < 0x40) pad->button |= D_CBUTTONS;
            // if (axis[3] > 0xC0) pad->button |= U_CBUTTONS;
            // int8_t stick_x = saturate(axis[0] - 128 - 0);
            // int8_t stick_y = saturate(axis[1] - 128 - 0);
            // if (stick_x != 0 || stick_y != 0) {
            //     pad->stick_x = stick_x;
            //     pad->stick_y = stick_y;
            // }

            unsafe {
                if gamepad.is_pressed(Button::Start) {
                    (*pad).button |= N64Button::Start as u16;
                }
                if gamepad.is_pressed(Button::LeftTrigger) {
                    (*pad).button |= N64Button::L as u16;
                }
                if gamepad.is_pressed(Button::RightTrigger) {
                    (*pad).button |= N64Button::R as u16;
                }
                if gamepad.is_pressed(Button::East) {
                    (*pad).button |= N64Button::A as u16;
                }
                if gamepad.is_pressed(Button::South) {
                    (*pad).button |= N64Button::B as u16;
                }

                let left_x = gamepad.value(Axis::LeftStickX);
                let left_y = gamepad.value(Axis::LeftStickY);
                let right_x = gamepad.value(Axis::RightStickX);
                let right_y = gamepad.value(Axis::RightStickY);

                // if (rightx < -0x4000) pad->button |= L_CBUTTONS;
                // if (rightx > 0x4000) pad->button |= R_CBUTTONS;
                // if (righty < -0x4000) pad->button |= U_CBUTTONS;
                // if (righty > 0x4000) pad->button |= D_CBUTTONS;

                // if (ltrig > 30 * 256) pad->button |= Z_TRIG;
                // if (rtrig > 30 * 256) pad->button |= R_TRIG;

                trace!("Left X: {}", left_x);
                trace!("Left Y: {}", left_x);
                trace!("Right X: {}", right_x);
                trace!("Right Y: {}", right_y);
            }
        }

        panic!("The given gamepad does not belong to this service");
    }
}
