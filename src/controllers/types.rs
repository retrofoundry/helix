use std::collections::HashMap;

pub enum N64Button {
    A, B,
    L, R, Z, Start,
    CUp, CDown, CLeft, CRight,
    DUp, DDown, DLeft, DRight
}

pub struct RumbleSettings {
    enabled: bool,
    strength: f32
}

pub struct Profile {
    version: u32,
    rumble: RumbleSettings,
    buttons: HashMap<N64Button, u64>
}