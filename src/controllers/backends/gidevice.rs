use std::cell::Cell;

use crate::helix;
use gilrs::{Gilrs, GamepadId};
use crate::controllers::types::{Profile};
use crate::controllers::device::ControllerDevice;

pub struct GIController {
    id: GamepadId,
    api: Cell<Gilrs>,
}

impl ControllerDevice for GIController {

    fn connected(&self) -> bool {
        true
    }

    fn read(&mut self) {
        let api = self.api.get_mut();
        api.next_event();
        if let Some(gamepad) = Some(self.id).map(|id| api.gamepad(id)) {
            for btn in gamepad.state().buttons() {
                print!("{:?} ", btn.0);
            }
        }
    }

    fn write(&mut self) {
        todo!()
    }

    fn load_profile(&self, slot: u8) -> &Profile {
        todo!()
    }

    fn scan(&mut self){
        let api = self.api.get_mut();
        for (id, _) in api.gamepads() {
            helix!().controller.register(Box::new(GIController {
                id,
                api: Cell::new(Gilrs::new().unwrap()),
            }));
        }
    }
}