use super::{types::Profile, hub::ControllerHub};

pub trait ControllerDevice {
    fn connected(&self) -> bool;
    fn read(&mut self);
    fn write(&mut self);
    fn load_profile(&self, slot: u8) -> &Profile;
    fn scan(&mut self);
}