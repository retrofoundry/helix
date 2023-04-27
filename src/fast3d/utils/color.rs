pub struct R5G5B5A1 {}

impl R5G5B5A1 {
    #[inline]
    pub fn to_rgba(pixel: u16) -> Color {
        let r = ((pixel & 0xF800) >> 11) as u8;
        let g = ((pixel & 0x07C0) >> 6) as u8;
        let b = ((pixel & 0x003E) >> 1) as u8;
        let a = (pixel & 0x01) as u8;

        Color::RGBA(r * 8, g * 8, b * 8, a * 255)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    #[inline]
    #[allow(non_snake_case)]
    pub const fn RGBA(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }
}
