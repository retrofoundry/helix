pub type GamepadBits = *mut u8;

pub enum N64Button {
    A = 0x8000,
    B = 0x4000,
    L = 0x0020,
    R = 0x0010,
    Z = 0x2000,
    Start = 0x1000,
    CUp = 0x0008,
    CDown = 0x0004,
    CLeft = 0x0002,
    CRight = 0x0001,
    DUp = 0x0800,
    DDown = 0x0400,
    DLeft = 0x0200,
    DRight = 0x0100,
}

// MARK: - [Libultra] - C API

#[repr(C)]
pub struct OSControllerPad {
    pub button: u16,
    pub stick_x: i8, /* -80 <= stick_x <= 80 */
    pub stick_y: i8, /* -80 <= stick_x <= 80 */
    pub errno: u8,
}
