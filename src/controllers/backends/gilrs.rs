use gilrs::{Gilrs, Button, Event, GamepadId};
use crate::controllers::types::{Profile};
use crate::controllers::device::ControllerDevice;

pub struct GIController {
    id: GamepadId,
    api: Gilrs
}

impl ControllerDevice for GIController {
    type DeviceID = GamepadId;

    fn new(device_id: Self::DeviceID) -> Self {
        GIController {
            id: device_id,
            api: Gilrs::new().unwrap()
        }
    }

    fn connected(&self) -> bool {
        true
    }

    fn read(&mut self) {
        self.api.next_event();

        if let Some(gamepad) = Some(self.id).map(|id| self.api.gamepad(id)) {
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
}