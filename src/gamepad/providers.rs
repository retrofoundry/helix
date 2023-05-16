use crate::gamepad::types::OSContPad;
use ::gilrs::GamepadId;

pub mod gilrs;

pub enum ControllerService {
    GilRs(GamepadId),
    Keyboard(),
}

pub trait ControllerProvider {
    fn scan(&self) -> Vec<Controller>;
    fn read(&self, controllers: &Controller, pad: *mut OSContPad);
}

pub struct Controller {
    pub _slot: u8,
    pub service: ControllerService,
}

impl Controller {
    pub fn new(service: ControllerService) -> Self {
        Self { _slot: 0, service }
    }
}
