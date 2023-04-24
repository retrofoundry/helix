// TODO: Expose this on Farbe
pub struct R5G5B5A1 {}

impl R5G5B5A1 {
    #[inline]
    pub fn to_rgba(pixel: u16) -> Vec<u8> {
        let r = ((pixel & 0xF800) >> 11) as u8;
        let g = ((pixel & 0x07C0) >> 6) as u8;
        let b = ((pixel & 0x003E) >> 1) as u8;
        let a = (pixel & 0x01) as u8;

        vec![r * 8, g * 8, b * 8, a * 255]
    }
}