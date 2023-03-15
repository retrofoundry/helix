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
    stype: u16,
    status: u8,
    errno: u8,
}

#[repr(C)]
pub struct OSContPad {
    button: u16,
    stick_x: i8,
    stick_y: i8,
    errno: u8,
}