use glam::Vec4;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    #[inline]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
}

impl From<Color> for Vec4 {
    fn from(c: Color) -> Self {
        Self::new(c.r, c.g, c.b, c.a)
    }
}

pub struct R5G5B5A1 {}

impl R5G5B5A1 {
    #[inline]
    pub fn to_rgba(pixel: u16) -> Color {
        let r = ((pixel & 0xF800) >> 11) as u8;
        let g = ((pixel & 0x07C0) >> 6) as u8;
        let b = ((pixel & 0x003E) >> 1) as u8;
        let a = (pixel & 0x01) as u8;

        Color::new(r as f32 / 31.0, g as f32 / 31.0, b as f32 / 31.0, a as f32)
    }
}
