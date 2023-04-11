#[repr(C)]
#[derive(Clone, Copy)]
pub struct GWords {
    pub w0: usize,
    pub w1: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Gfx {
    pub words: GWords,
    pub force_structure_alignment: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Viewport {
    pub vscale: [i16; 4], // scale, 2 bits fraction
    pub vtrans: [i16; 4], // translate, 2 bits fraction
    _padding: [u8; 8],    // padding to 64-bit boundary
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Light {
    pub col: [u8; 3],
    pad1: i8,
    pub colc: [u8; 3],
    pad2: i8,
    pub dir: [i8; 3],
    pad3: i8,
}

impl Light {
    pub const ZERO: Self = Self {
        col: [0, 0, 0],
        pad1: 0,
        colc: [0, 0, 0],
        pad2: 0,
        dir: [0, 0, 0],
        pad3: 0,
    };
}

#[cfg(feature = "f3dex2")]
#[derive(PartialEq, Eq)]
pub enum G_MTX {
    NOPUSH_MUL_MODELVIEW = 0x00,
    PUSH = 0x01,
    // MUL = 0x00,
    LOAD = 0x02,
    // MODELVIEW = 0x00,
    PROJECTION = 0x04,
}

#[cfg(feature = "f3dex2")]
#[derive(PartialEq, Eq)]
pub enum G_MV {
    MMTX = 2,
    PMTX = 6,
    VIEWPORT = 8,
    LIGHT = 10,
    POINT = 12,
}
