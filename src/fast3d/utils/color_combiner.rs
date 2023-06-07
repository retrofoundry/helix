use crate::fast3d::gbi::utils::get_cmd;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AlphaCombinePass {
    pub a: ACMUX,
    pub b: ACMUX,
    pub c: ACMUX,
    pub d: ACMUX,
}

impl AlphaCombinePass {
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CombineParams {
    pub c0: ColorCombinePass,
    pub a0: AlphaCombinePass,
    pub c1: ColorCombinePass,
    pub a1: AlphaCombinePass,
}

impl CombineParams {
    pub const ZERO: Self = Self {
        c0: ColorCombinePass {
            a: CCMUX::COMBINED,
            b: CCMUX::TEXEL0,
            c: CCMUX::PRIMITIVE,
            d: CCMUX::COMBINED,
        },
        a0: AlphaCombinePass {
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
        a1: AlphaCombinePass {
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
            a0: AlphaCombinePass {
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
            a1: AlphaCombinePass {
                a: ACMUX::from(aa1),
                b: ACMUX::from(ab1),
                c: ACMUX::from(ac1),
                d: ACMUX::from(ad1),
            },
        }
    }

    pub fn get_cc(&self, index: usize) -> ColorCombinePass {
        match index {
            0 => self.c0,
            1 => self.c1,
            _ => panic!("Invalid index"),
        }
    }

    pub fn get_ac(&self, index: usize) -> AlphaCombinePass {
        match index {
            0 => self.a0,
            1 => self.a1,
            _ => panic!("Invalid index"),
        }
    }

    pub fn cc_ac_same(&self, index: usize) -> bool {
        match index {
            0 => {
                self.c0.a as u8 == self.a0.a as u8
                    && self.c0.b as u8 == self.a0.b as u8
                    && self.c0.c as u8 == self.a0.c as u8
                    && self.c0.d as u8 == self.a0.d as u8
            }
            1 => {
                self.c1.a as u8 == self.a1.a as u8
                    && self.c1.b as u8 == self.a1.b as u8
                    && self.c1.c as u8 == self.a1.c as u8
                    && self.c1.d as u8 == self.a1.d as u8
            }
            _ => panic!("Invalid index"),
        }
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
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
