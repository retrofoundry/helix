use super::types::Profile;

pub trait ControllerDevice {
    type DeviceID;
    fn new(id: Self::DeviceID) -> Self;
    fn connected(&self) -> bool;
    fn read(&mut self);
    fn write(&mut self);
    fn load_profile(&self, slot: u8) -> &Profile;
}