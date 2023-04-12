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
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub vscale: [i16; 4], // scale, 2 bits fraction
    pub vtrans: [i16; 4], // translate, 2 bits fraction
    _padding: [u8; 8],    // padding to 64-bit boundary
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Light {
    pub col: [u8; 3], // diffuse light value (rgba)
    pad1: i8,
    pub colc: [u8; 3], // copy of diffuse light value (rgba)
    pad2: i8,
    pub dir: [i8; 3], // direction of light (normalized)
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

    pub const fn new(col: [u8; 3], colc: [u8; 3], dir: [i8; 3]) -> Self {
        Self {
            col,
            pad1: 0,
            colc,
            pad2: 0,
            dir,
            pad3: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Vtx {
    pub vertex: Vtx_t,
    pub normal: Vtx_tn,
    force_structure_alignment: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vtx_t {
    pub position: [i16; 3], // in object space
    flag: u16,              // unused
    pub texture_coords: [i16; 2],
    pub color: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vtx_tn {
    pub position: [i16; 3], // in object space
    flag: u16,              // unused
    pub texture_coords: [i16; 2],
    pub normal: [i8; 3],
    pub alpha: u8,
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

/*
 * MOVEMEM indices
 *
 * Each of these indexes an entry in a dmem table
 * which points to a 1-4 word block of dmem in
 * which to store a 1-4 word DMA.
 *
 */
#[cfg(feature = "f3dex2")]
#[derive(PartialEq, Eq)]
pub enum G_MV {
    MMTX = 2,
    PMTX = 6,
    VIEWPORT = 8,
    LIGHT = 10,
    POINT = 12,
    O_LOOKATX = 0 * 24,
    O_LOOKATY = 1 * 24,
    O_L0 = 2 * 24,
    O_L1 = 3 * 24,
    O_L2 = 4 * 24,
    O_L3 = 5 * 24,
    O_L4 = 6 * 24,
    O_L5 = 7 * 24,
    O_L6 = 8 * 24,
    O_L7 = 9 * 24,
}

/*
 * MOVEWORD indices
 *
 * Each of these indexes an entry in a dmem table
 * which points to a word in dmem in dmem where
 * an immediate word will be stored.
 *
 */
#[derive(PartialEq, Eq)]
pub enum G_MW {
    MATRIX = 0x00, /* NOTE: also used by movemem */
    NUMLIGHT = 0x02,
    CLIP = 0x04,
    SEGMENT = 0x06,
    FOG = 0x08,
    LIGHTCOL = 0x0A,
    #[cfg(feature = "f3dex2")]
    FORCEMTX = 0x0C,
    #[cfg(not(feature = "f3dex2"))]
    POINTS = 0x0C,
    PERSPNORM = 0x0E,
}
