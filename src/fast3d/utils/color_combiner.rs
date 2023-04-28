use crate::fast3d::{gbi::utils::get_cmd, graphics::ShaderProgram, rcp::RCP};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorCombinePass {
    pub a: CCMUX,
    pub b: CCMUX,
    pub c: CCMUX,
    pub d: CCMUX,
}

impl ColorCombinePass {
    // grab property by index
    pub fn get(&self, index: usize) -> CCMUX {
        match index {
            0 => self.a,
            1 => self.b,
            2 => self.c,
            3 => self.d,
            _ => panic!("Invalid index"),
        }
    }

    pub fn uses_texture0(&self) -> bool {
        self.a == CCMUX::TEXEL0
            || self.a == CCMUX::TEXEL0_ALPHA
            || self.b == CCMUX::TEXEL0
            || self.b == CCMUX::TEXEL0_ALPHA
            || self.c == CCMUX::TEXEL0
            || self.c == CCMUX::TEXEL0_ALPHA
            || self.d == CCMUX::TEXEL0
            || self.d == CCMUX::TEXEL0_ALPHA
    }

    pub fn uses_texture1(&self) -> bool {
        self.a == CCMUX::TEXEL1
            || self.a == CCMUX::TEXEL1_ALPHA
            || self.b == CCMUX::TEXEL1
            || self.b == CCMUX::TEXEL1_ALPHA
            || self.c == CCMUX::TEXEL1
            || self.c == CCMUX::TEXEL1_ALPHA
            || self.d == CCMUX::TEXEL1
            || self.d == CCMUX::TEXEL1_ALPHA
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlhpaCombinePass {
    a: ACMUX,
    b: ACMUX,
    c: ACMUX,
    d: ACMUX,
}

impl AlhpaCombinePass {
    // grab property by index
    pub fn get(&self, index: usize) -> ACMUX {
        match index {
            0 => self.a,
            1 => self.b,
            2 => self.c,
            3 => self.d,
            _ => panic!("Invalid index"),
        }
    }

    pub fn uses_texture0(&self) -> bool {
        self.a == ACMUX::TEXEL0
            || self.b == ACMUX::TEXEL0
            || self.c == ACMUX::TEXEL0
            || self.d == ACMUX::TEXEL0
    }

    pub fn uses_texture1(&self) -> bool {
        self.a == ACMUX::TEXEL1
            || self.b == ACMUX::TEXEL1
            || self.c == ACMUX::TEXEL1
            || self.d == ACMUX::TEXEL1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombineParams {
    pub c0: ColorCombinePass,
    pub a0: AlhpaCombinePass,
    c1: ColorCombinePass,
    a1: AlhpaCombinePass,
}

impl CombineParams {
    pub const ZERO: Self = Self {
        c0: ColorCombinePass {
            a: CCMUX::COMBINED,
            b: CCMUX::TEXEL0,
            c: CCMUX::PRIMITIVE,
            d: CCMUX::COMBINED,
        },
        a0: AlhpaCombinePass {
            a: ACMUX::COMBINED__LOD_FRAC,
            b: ACMUX::TEXEL0,
            c: ACMUX::PRIMITIVE,
            d: ACMUX::COMBINED__LOD_FRAC,
        },
        c1: ColorCombinePass {
            a: CCMUX::COMBINED,
            b: CCMUX::TEXEL0,
            c: CCMUX::PRIMITIVE,
            d: CCMUX::COMBINED,
        },
        a1: AlhpaCombinePass {
            a: ACMUX::COMBINED__LOD_FRAC,
            b: ACMUX::TEXEL0,
            c: ACMUX::PRIMITIVE,
            d: ACMUX::COMBINED__LOD_FRAC,
        },
    };

    pub fn decode(w0: usize, w1: usize) -> Self {
        let a0 = (get_cmd(w0, 20, 4) & 0xF) as u8;
        let b0 = (get_cmd(w1, 28, 4) & 0xF) as u8;
        let c0 = (get_cmd(w0, 15, 5) & 0x1F) as u8;
        let d0 = (get_cmd(w1, 15, 3) & 0x7) as u8;

        let aa0 = (get_cmd(w0, 12, 3) & 0x7) as u8;
        let ab0 = (get_cmd(w1, 12, 3) & 0x7) as u8;
        let ac0 = (get_cmd(w0, 9, 3) & 0x7) as u8;
        let ad0 = (get_cmd(w1, 9, 3) & 0x7) as u8;

        let a1 = (get_cmd(w0, 5, 4) & 0xF) as u8;
        let b1 = (get_cmd(w1, 24, 4) & 0xF) as u8;
        let c1 = (get_cmd(w0, 0, 5) & 0x1F) as u8;
        let d1 = (get_cmd(w1, 6, 3) & 0x7) as u8;

        let aa1 = (get_cmd(w1, 21, 3) & 0x7) as u8;
        let ab1 = (get_cmd(w1, 3, 3) & 0x7) as u8;
        let ac1 = (get_cmd(w1, 18, 3) & 0x7) as u8;
        let ad1 = (get_cmd(w1, 0, 3) & 0x7) as u8;

        Self {
            c0: ColorCombinePass {
                a: CCMUX::from(a0),
                b: CCMUX::from(b0),
                c: CCMUX::from(c0),
                d: CCMUX::from(d0),
            },
            a0: AlhpaCombinePass {
                a: ACMUX::from(aa0),
                b: ACMUX::from(ab0),
                c: ACMUX::from(ac0),
                d: ACMUX::from(ad0),
            },
            c1: ColorCombinePass {
                a: CCMUX::from(a1),
                b: CCMUX::from(b1),
                c: CCMUX::from(c1),
                d: CCMUX::from(d1),
            },
            a1: AlhpaCombinePass {
                a: ACMUX::from(aa1),
                b: ACMUX::from(ab1),
                c: ACMUX::from(ac1),
                d: ACMUX::from(ad1),
            },
        }
    }

    pub fn to_u32(&self) -> u32 {
        let c0 = self.c0;
        let a0 = self.a0;

        let cout =
            (c0.a as u32) | ((c0.b as u32) << 3) | ((c0.c as u32) << 6) | ((c0.d as u32) << 9);
        let aout =
            (a0.a as u32) | ((a0.b as u32) << 3) | ((a0.c as u32) << 6) | ((a0.d as u32) << 9);

        cout | (aout << 12)
    }

    pub fn uses_texture0(&self) -> bool {
        self.c0.uses_texture0()
            || self.c1.uses_texture0()
            || self.a0.uses_texture0()
            || self.a1.uses_texture0()
    }

    pub fn uses_texture1(&self) -> bool {
        self.c0.uses_texture1()
            || self.c1.uses_texture1()
            || self.a0.uses_texture1()
            || self.a1.uses_texture1()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub enum CCMUX {
    COMBINED = 0,
    TEXEL0 = 1,
    TEXEL1 = 2,
    PRIMITIVE = 3,
    SHADE = 4,
    ENVIRONMENT = 5,
    CENTER__SCALE__ONE = 6,
    // param C only
    COMBINED_ALPHA__NOISE__K4 = 7, // COMBINE_A only for C (ADD_ZERO?)
    TEXEL0_ALPHA = 8,
    TEXEL1_ALPHA = 9,
    PRIMITIVE_ALPHA = 10,
    SHADE_ALPHA = 11,
    ENV_ALPHA = 12,
    LOD_FRACTION = 13,
    PRIM_LOD_FRACTION = 14,
    K5 = 15, // MUL_ZERO?
    ZERO = 31,
}

impl CCMUX {
    pub fn from(val: u8) -> Self {
        match val {
            0 => CCMUX::COMBINED,
            1 => CCMUX::TEXEL0,
            2 => CCMUX::TEXEL1,
            3 => CCMUX::PRIMITIVE,
            4 => CCMUX::SHADE,
            5 => CCMUX::ENVIRONMENT,
            6 => CCMUX::CENTER__SCALE__ONE,
            7 => CCMUX::COMBINED_ALPHA__NOISE__K4,
            8 => CCMUX::TEXEL0_ALPHA,
            9 => CCMUX::TEXEL1_ALPHA,
            10 => CCMUX::PRIMITIVE_ALPHA,
            11 => CCMUX::SHADE_ALPHA,
            12 => CCMUX::ENV_ALPHA,
            13 => CCMUX::LOD_FRACTION,
            14 => CCMUX::PRIM_LOD_FRACTION,
            15 => CCMUX::K5,
            31 => CCMUX::ZERO,
            _ => panic!("Invalid CCMUX value: {}", val),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub enum ACMUX {
    COMBINED__LOD_FRAC = 0, // ADD?
    TEXEL0 = 1,
    TEXEL1 = 2,
    PRIMITIVE = 3,
    SHADE = 4,
    ENVIRONMENT = 5,
    PRIM_LOD_FRAC__ONE = 6,
    ZERO = 7,
}

impl ACMUX {
    pub fn from(val: u8) -> Self {
        match val {
            0 => ACMUX::COMBINED__LOD_FRAC,
            1 => ACMUX::TEXEL0,
            2 => ACMUX::TEXEL1,
            3 => ACMUX::PRIMITIVE,
            4 => ACMUX::SHADE,
            5 => ACMUX::ENVIRONMENT,
            6 => ACMUX::PRIM_LOD_FRAC__ONE,
            7 => ACMUX::ZERO,
            _ => panic!("Invalid ACMUX value: {}", val),
        }
    }
}

pub const SHADER_OPT_ALPHA: u32 = 1 << 24;
pub const SHADER_OPT_FOG: u32 = 1 << 25;
pub const SHADER_OPT_TEXTURE_EDGE: u32 = 1 << 26;
pub const SHADER_OPT_NOISE: u32 = 1 << 27;

pub enum SHADER {
    ZERO,
    INPUT_1,
    INPUT_2,
    INPUT_3,
    INPUT_4,
    TEXEL0,
    TEXEL0A,
    TEXEL1,
}

pub struct ColorCombinerManager {
    pub combiners: HashMap<u32, ColorCombiner>,
    pub current_combiner: Option<u32>,
}

impl ColorCombinerManager {
    pub fn new() -> Self {
        Self {
            combiners: HashMap::new(),
            current_combiner: None,
        }
    }

    pub fn lookup_color_combiner(&mut self, cc_id: u32) -> Option<&ColorCombiner> {
        if let Some(current_cc_id) = self.current_combiner {
            if current_cc_id == cc_id {
                if let Some(cc) = self.combiners.get(&cc_id) {
                    return Some(cc);
                }
            }
        }

        if let Some(cc) = self.combiners.get(&cc_id) {
            self.current_combiner = Some(cc_id);
            return Some(cc);
        }

        None
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColorCombiner {
    pub cc_id: u32,
    pub prg: *mut ShaderProgram,
    pub shader_input_mapping: [[u8; 4]; 2],
}

impl ColorCombiner {
    pub fn new(
        shader_id: u32,
        shader_program: *mut ShaderProgram,
        shader_input_mapping: [[u8; 4]; 2],
    ) -> Self {
        Self {
            cc_id: shader_id,
            prg: shader_program,
            shader_input_mapping,
        }
    }
}

// MARK: - C Bridge

#[no_mangle]
pub extern "C" fn RDPGetColorCombiner(rcp: Option<&mut RCP>, cc_id: u32) -> *const ColorCombiner {
    let rcp = rcp.unwrap();
    let color_combiner = rcp
        .rdp
        .color_combiner_manager
        .combiners
        .get_mut(&cc_id)
        .unwrap();
    color_combiner as *const ColorCombiner
}
