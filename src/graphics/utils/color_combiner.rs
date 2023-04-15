use crate::graphics::{
    gfx_device::{GfxDevice, ShaderProgram},
    rcp::RCP,
};
use std::collections::HashMap;

#[derive(PartialEq, Eq)]
pub enum CC {
    NIL,
    TEXEL0,
    TEXEL1,
    PRIM,
    SHADE,
    ENV,
    TEXEL0A,
    LOD,
}

impl CC {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            x if x == CC::NIL as u8 => Some(CC::NIL),
            x if x == CC::TEXEL0 as u8 => Some(CC::TEXEL0),
            x if x == CC::TEXEL1 as u8 => Some(CC::TEXEL1),
            x if x == CC::PRIM as u8 => Some(CC::PRIM),
            x if x == CC::SHADE as u8 => Some(CC::SHADE),
            x if x == CC::ENV as u8 => Some(CC::ENV),
            x if x == CC::TEXEL0A as u8 => Some(CC::TEXEL0A),
            x if x == CC::LOD as u8 => Some(CC::LOD),
            _ => None,
        }
    }
}

pub enum SHADER {
    NIL,
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
    shader_input_mapping: [[u8; 4]; 2],
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
