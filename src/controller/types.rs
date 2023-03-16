use std::collections::HashMap;

pub type ControllerBits = Box<u8>;

pub enum N64Button {
    A,
    B,
    L,
    R,
    Z,
    Start,
    CUp,
    CDown,
    CLeft,
    CRight,
    DUp,
    DDown,
    DLeft,
    DRight,
}

pub struct RumbleSettings {
    enabled: bool,
    strength: f32,
}

pub struct Profile {
    version: u32,
    rumble: RumbleSettings,
    buttons: HashMap<N64Button, u64>,
}

// MARK: - [Libultra] - C API

#[repr(C)]
pub struct OSContStatus {
    pub stype: u16,
    pub status: u8,
    pub errno: u8,
}

#[repr(C)]
pub struct OSContPad {
    pub button: u16,
    pub stick_x: i8,
    pub stick_y: i8,
    pub errno: u8,
}
