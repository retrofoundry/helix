use crate::gamepad::types::OSControllerPad;
use ::gilrs::GamepadId;
use winit::event::{KeyboardInput, ModifiersState};

pub mod gilrs;
pub mod keyboard;

pub enum GamepadService {
    GilRs(GamepadId),
    Keyboard(),
}

pub trait GamepadProvider {
    fn scan(&self) -> Vec<Gamepad>;
    fn process_events(&mut self);
    unsafe fn read(&self, controllers: &Gamepad, pad: *mut OSControllerPad);

    fn handle_keyboard_input(&mut self, input: KeyboardInput);
    fn handle_modifiers_changed(&mut self, modifiers: ModifiersState);
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
