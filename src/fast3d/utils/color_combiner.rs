use crate::fast3d::gbi::utils::get_cmd;

#[derive(Debug)]
pub struct ShaderInputMapping {
    pub num_inputs: u8,
    pub mirror_mapping: [[SHADER; 4]; 2],
    pub input_mapping: [[u8; 4]; 2],
}

impl ShaderInputMapping {
    pub const ZERO: Self = Self {
        num_inputs: 0,
        mirror_mapping: [[SHADER::ZERO; 4]; 2],
        input_mapping: [[0; 4]; 2],
    };
}

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
    c1: ColorCombinePass,
    a1: AlphaCombinePass,
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

    pub fn shader_input_mapping(&self) -> ShaderInputMapping {
        let mut num_inputs = 0;
        let mut mirror_mapping = [[SHADER::ZERO; 4]; 2];
        let mut input_mapping = [[0u8; 4]; 2];

        let mut cc_input_number = [0u8; 8];
        let mut cc_next_input_number = SHADER::INPUT1 as u8;

        let mut ac_input_number = [0u8; 8];
        let mut ac_next_input_number = SHADER::INPUT1 as u8;

        for i in 0..2 {
            match i % 2 {
                0 => {
                    for j in 0..4 {
                        let property = self.get_cc(i / 2).get(j);
                        match property {
                            CCMUX::ZERO => mirror_mapping[i][j] = SHADER::ZERO,
                            CCMUX::TEXEL0 => mirror_mapping[i][j] = SHADER::TEXEL0,
                            CCMUX::TEXEL1 => mirror_mapping[i][j] = SHADER::TEXEL1,
                            CCMUX::TEXEL0_ALPHA => mirror_mapping[i][j] = SHADER::TEXEL0A,
                            CCMUX::PRIMITIVE | CCMUX::SHADE | CCMUX::ENVIRONMENT | CCMUX::LOD_FRACTION => {
                                if cc_input_number[property as usize] == 0 {
                                    input_mapping[i][(cc_next_input_number - 1) as usize] = property as u8;
                                    cc_input_number[property as usize] = cc_next_input_number;

                                    mirror_mapping[i][j] =
                                        SHADER::from(cc_next_input_number);

                                    if mirror_mapping[i][j] >= SHADER::INPUT1
                                        && mirror_mapping[i][j] <= SHADER::INPUT4
                                        && mirror_mapping[i][j] as u8 > num_inputs
                                    {
                                        num_inputs = cc_next_input_number;
                                    }

                                    cc_next_input_number += 1;
                                }
                            }
                            _ => {
                                // panic!("Invalid CCMUX value: {:?}", property)
                            }
                        }
                    }
                }
                1 => {
                    for j in 0..4 {
                        let property = self.get_ac((i - 1) / 2).get(j);
                        match property {
                            ACMUX::ZERO => mirror_mapping[i][j] = SHADER::ZERO,
                            ACMUX::TEXEL0 => mirror_mapping[i][j] = SHADER::TEXEL0,
                            ACMUX::TEXEL1 => mirror_mapping[i][j] = SHADER::TEXEL1,
                            ACMUX::PRIMITIVE | ACMUX::SHADE | ACMUX::ENVIRONMENT => {
                                if ac_input_number[property as usize] == 0 {
                                    input_mapping[i][(ac_next_input_number - 1) as usize] = property as u8;
                                    ac_input_number[property as usize] = ac_next_input_number;
                                    ac_next_input_number += 1;

                                    mirror_mapping[i][j] =
                                        SHADER::from(ac_input_number[property as usize]);

                                    if mirror_mapping[i][j] >= SHADER::INPUT1
                                        && mirror_mapping[i][j] <= SHADER::INPUT4
                                        && mirror_mapping[i][j] as u8 > num_inputs
                                    {
                                        num_inputs = ac_input_number[property as usize];
                                    }
                                }
                            }
                            _ => {
                                // panic!("Invalid ACMUX value: {:?}", property)
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }

        ShaderInputMapping {
            num_inputs,
            mirror_mapping,
            input_mapping,
        }
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum SHADER {
    ZERO,
    INPUT1,
    INPUT2,
    INPUT3,
    INPUT4,
    TEXEL0,
    TEXEL0A,
    TEXEL1,
}

impl SHADER {
    pub fn from(val: u8) -> Self {
        match val {
            0 => SHADER::ZERO,
            1 => SHADER::INPUT1,
            2 => SHADER::INPUT2,
            3 => SHADER::INPUT3,
            4 => SHADER::INPUT4,
            5 => SHADER::TEXEL0,
            6 => SHADER::TEXEL0A,
            7 => SHADER::TEXEL1,
            _ => panic!("Invalid SHADER value: {}", val),
        }
    }
}
