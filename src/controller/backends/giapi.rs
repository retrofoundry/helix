use gilrs::{GamepadId, Gilrs};
use std::sync::{Arc, Mutex};

use crate::controller::types::OSContPad;

use super::super::device::ControllerDevice;
use super::super::types::Profile;

pub struct GIApi {
    pub api: Arc<Mutex<Gilrs>>,
}

impl GIApi {
    pub fn new() -> GIApi {
        GIApi {
            api: Arc::new(Mutex::new(Gilrs::new().unwrap())),
        }
    }

    pub fn scan(&self) -> Vec<Box<dyn ControllerDevice>> {
        let mut devices: Vec<Box<dyn ControllerDevice>> = Vec::new();

        for (id, _gamepad) in self.api.lock().unwrap().gamepads() {
            let api = Arc::clone(&self.api);
            devices.push(Box::new(GIController { id, api }));
        }

        devices
    }
}

pub struct GIController {
    pub id: GamepadId,
    pub api: Arc<Mutex<Gilrs>>,
}

impl ControllerDevice for GIController {
    fn connected(&self) -> bool {
        true
    }

    fn read(&mut self) {
        let mut api = self.api.lock().unwrap();
        api.next_event();

        if let Some(gamepad) = Some(self.id).map(|id| api.gamepad(id)) {
            for btn in gamepad.state().buttons() {
                print!("{:?} ", btn.0);
            }
        }
    }

    fn write(&mut self, data: &OSContPad) {
        if data.stick_x != 0 {
            data.stick_x = 0;
        }
        if data.stick_y != 0 {
            data.stick_y = 0;
        }
    }

    fn load_profile(&self, _slot: u8) -> &Profile {
        todo!()
    }
}
