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
pub struct RawLight {
    pub words: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Light {
    pub raw: RawLight,
    pub pos: PosLight,
    pub dir: DirLight,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DirLight {
    pub col: [u8; 3], // diffuse light value (rgba)
    pad1: i8,
    pub colc: [u8; 3], // copy of diffuse light value (rgba)
    pad2: i8,
    pub dir: [i8; 3], // direction of light (normalized)
    pad3: i8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PosLight {
    pub kc: u8,
    pub col: [u8; 3],
    pub kl: u8,
    pub colc: [u8; 3],
    pub pos: [i16; 2],
    pub reserved1: u8,
    pub kq: u8,
    pub posz: i16,
}

impl Light {
    pub const ZERO: Self = Self {
        raw: RawLight {
            words: [0, 0, 0, 0],
        },
    };
}

pub struct LookAt {
    pub x: [f32; 3],
    pub y: [f32; 3],
}

impl LookAt {
    pub const fn new(x: [f32; 3], y: [f32; 3]) -> Self {
        Self { x, y }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Vtx {
    pub vertex: Vtx_t,
    pub normal: Vtx_tn,
    force_structure_alignment: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vtx_t {
    #[cfg(feature = "gbifloats")]
    pub position: [f32; 3], // in object space
    #[cfg(not(feature = "gbifloats"))]
    pub position: [i16; 3], // in object space
    flag: u16, // unused
    pub texture_coords: [i16; 2],
    pub color: Color_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vtx_tn {
    #[cfg(feature = "gbifloats")]
    pub position: [f32; 3], // in object space
    #[cfg(not(feature = "gbifloats"))]
    pub position: [i16; 3], // in object space
    flag: u16, // unused
    pub texture_coords: [i16; 2],
    pub normal: [i8; 3],
    pub alpha: u8,
}

#[cfg(feature = "f3dex2")]
pub struct G_MTX;
#[cfg(feature = "f3dex2")]
impl G_MTX {
    pub const NOPUSH: u8 = 0x00;
    pub const PUSH: u8 = 0x01;
    pub const MUL: u8 = 0x00;
    pub const LOAD: u8 = 0x02;
    pub const MODELVIEW: u8 = 0x00;
    pub const PROJECTION: u8 = 0x04;
}

pub struct G_SET;
impl G_SET {
    pub const COLORIMG: u8 = 0xff;
    pub const DEPTHIMG: u8 = 0xfe;
    pub const TEXIMG: u8 = 0xfd;
    pub const COMBINE: u8 = 0xfc;
    pub const ENVCOLOR: u8 = 0xfb;
    pub const PRIMCOLOR: u8 = 0xfa;
    pub const BLENDCOLOR: u8 = 0xf9;
    pub const FOGCOLOR: u8 = 0xf8;
    pub const FILLCOLOR: u8 = 0xf7;
    pub const TILE: u8 = 0xf5;
    pub const TILESIZE: u8 = 0xf2;
    pub const PRIMDEPTH: u8 = 0xee;
    pub const SCISSOR: u8 = 0xed;
    pub const CONVERT: u8 = 0xec;
    pub const KEYR: u8 = 0xeb;
    pub const KEYGB: u8 = 0xea;
}

pub struct G_LOAD;
impl G_LOAD {
    pub const BLOCK: u8 = 0xf3;
    pub const TILE: u8 = 0xf4;
    pub const TLUT: u8 = 0xf0;
}

pub struct G_MW;
impl G_MW {
    pub const MATRIX: u8 = 0x00; /* NOTE: also used by movemem */
    pub const NUMLIGHT: u8 = 0x02;
    pub const CLIP: u8 = 0x04;
    pub const SEGMENT: u8 = 0x06;
    pub const FOG: u8 = 0x08;
    pub const LIGHTCOL: u8 = 0x0A;
    #[cfg(feature = "f3dex2")]
    pub const FORCEMTX: u8 = 0x0C;
    #[cfg(not(feature = "f3dex2"))]
    pub const POINTS: u8 = 0x0C;
    pub const PERSPNORM: u8 = 0x0E;
}

pub struct G_TX;
impl G_TX {
    pub const LOADTILE: u8 = 7;
    pub const RENDERTILE: u8 = 0;
    pub const NOMIRROR: u8 = 0;
    pub const WRAP: u8 = 0;
    pub const MIRROR: u8 = 1;
    pub const CLAMP: u8 = 2;
    pub const NOMASK: u8 = 0;
    pub const NOLOD: u8 = 0;
}

// lose defines

pub const G_TEXRECT: u8 = 0xe4;
pub const G_TEXRECTFLIP: u8 = 0xe5;
pub const G_FILLRECT: u8 = 0xf6;

pub const G_RDPFULLSYNC: u8 = 0xe9;
pub const G_RDPTILESYNC: u8 = 0xe8;
pub const G_RDPPIPESYNC: u8 = 0xe7;
pub const G_RDPLOADSYNC: u8 = 0xe6;
