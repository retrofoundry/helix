use std::collections::HashMap;

struct RumbleSettings {
    enabled: bool,
    strength: f32
}

struct Profile {
    version: u32,
    rumble: RumbleSettings,
    buttons: HashMap,
}

pub trait ControllerDevice {
    fn new() -> Self;
    fn connected(&self) -> bool;
    fn read(&mut self);
    fn write(&mut self);
    fn load_profile(&self, slot: u8) -> &Profile;
}