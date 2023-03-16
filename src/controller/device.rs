use super::types::{OSContPad, Profile};

pub trait ControllerDevice {
    fn connected(&self) -> bool;
    fn read(&mut self);
    fn write(&mut self, data: &mut OSContPad);
    fn load_profile(&self, slot: u8) -> &Profile;
}
