use super::texture::{ImageFormat, ImageSize};

#[derive(Debug, Clone, Copy, Hash)]
pub struct TileDescriptor {
    pub uls: u16,
    pub ult: u16,
    pub lrs: u16,
    pub lrt: u16,
    // Set by G_SETTILE
    pub format: ImageFormat,
    pub size: ImageSize,
    /// Size of 1 line (s-axis) of texture tile (9bit precision, 0 - 511)
    pub line: u16,
    /// Address of texture tile origin (9bit precision, 0 - 511)
    pub tmem: u16,
    /// slot in tmem (usually 0 or 1)?
    pub tmem_index: u8,
    /// Position of palette for 4bit color index textures (4bit precision, 0 - 15)
    pub palette: u8,
    /// s-axis mirror, wrap, clamp flags
    pub cm_s: u8,
    /// s-axis mask (4bit precision, 0 - 15)
    pub mask_s: u8,
    /// s-coordinate shift value
    pub shift_s: u8,
    /// t-axis mirror, wrap, clamp flags
    pub cm_t: u8,
    /// t-axis mask (4bit precision, 0 - 15)
    pub mask_t: u8,
    /// t-coordinate shift value
    pub shift_t: u8,
}

impl TileDescriptor {
    pub const EMPTY: Self = Self {
        uls: 0,
        ult: 0,
        lrs: 0,
        lrt: 0,
        format: ImageFormat::G_IM_FMT_YUV,
        size: ImageSize::G_IM_SIZ_4b,
        line: 0,
        tmem: 0,
        tmem_index: 0,
        palette: 0,
        cm_s: 0,
        mask_s: 0,
        shift_s: 0,
        cm_t: 0,
        mask_t: 0,
        shift_t: 0,
    };

    pub fn set_format(&mut self, format: u8) {
        match format {
            0 => self.format = ImageFormat::G_IM_FMT_RGBA,
            1 => self.format = ImageFormat::G_IM_FMT_YUV,
            2 => self.format = ImageFormat::G_IM_FMT_CI,
            3 => self.format = ImageFormat::G_IM_FMT_IA,
            4 => self.format = ImageFormat::G_IM_FMT_I,
            _ => panic!("Invalid format: {}", format),
        }
    }

    pub fn set_size(&mut self, size: u8) {
        match size {
            0 => self.size = ImageSize::G_IM_SIZ_4b,
            1 => self.size = ImageSize::G_IM_SIZ_8b,
            2 => self.size = ImageSize::G_IM_SIZ_16b,
            3 => self.size = ImageSize::G_IM_SIZ_32b,
            _ => panic!("Invalid size: {}", size),
        }
    }

    pub fn get_width(&self) -> u16 {
        ((self.lrs - self.uls) + 4) / 4
    }

    pub fn get_height(&self) -> u16 {
        ((self.lrt - self.ult) + 4) / 4
    }
}
