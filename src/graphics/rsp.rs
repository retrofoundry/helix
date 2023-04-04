pub struct RSP {
    // Geometry Mode
    pub geometry_mode: u32,

    // State
    pub state_changed: bool,
}

impl RSP {
    pub fn new() -> Self {
        RSP {
            geometry_mode: 0,

            state_changed: false,
        }
    }

    pub fn reset(&mut self) {}
}
