use crate::gamepad::types::OSControllerPad;
use ::gilrs::GamepadId;

pub mod gilrs;

pub enum GamepadService {
    GilRs(GamepadId),
    Keyboard(),
}

pub trait GamepadProvider {
    fn scan(&self) -> Vec<Gamepad>;
    fn process_events(&mut self);
    fn read(&self, controllers: &Gamepad, pad: *mut OSControllerPad);
}

pub struct Gamepad {
    pub _slot: u8,
    pub service: GamepadService,
}

impl Gamepad {
    pub fn new(service: GamepadService) -> Self {
        Self { _slot: 0, service }
    }
}
