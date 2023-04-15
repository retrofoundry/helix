use crate::graphics::{gfx_device::ShaderProgram, rcp::RCP};
use log::trace;
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
    combiners: HashMap<u32, ColorCombiner>,
    current_combiner: Option<u32>,
}

impl ColorCombinerManager {
    pub fn new() -> Self {
        Self {
            combiners: HashMap::new(),
            current_combiner: None,
        }
    }

    pub fn add_combiner(&mut self, combiner: ColorCombiner) {
        self.current_combiner = Some(combiner.cc_id);
        self.combiners.insert(combiner.cc_id, combiner);
    }

    pub fn get_combiner(&mut self, cc_id: u32) -> Option<&mut ColorCombiner> {
        if let Some(cc) = self.combiners.get_mut(&cc_id) {
            self.current_combiner = Some(cc_id);
            trace!("Found combiner with id {}", cc_id);
            Some(cc)
        } else {
            trace!("No combiner with id {}", cc_id);
            None
        }
    }

    pub fn get_current_combiner(&mut self) -> Option<&mut ColorCombiner> {
        if let Some(cc_id) = self.current_combiner {
            self.combiners.get_mut(&cc_id)
        } else {
            None
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColorCombiner {
    cc_id: u32,
    prg: *mut ShaderProgram,
    shader_input_mapping: [[u8; 4]; 2],
}

impl ColorCombiner {
    pub const ZERO: Self = Self {
        cc_id: 0,
        prg: std::ptr::null_mut(),
        shader_input_mapping: [[0; 4]; 2],
    };

    pub fn new(shader_id: u32, shader_input_mapping: [[u8; 4]; 2]) -> Self {
        Self {
            cc_id: shader_id,
            prg: std::ptr::null_mut(),
            shader_input_mapping,
        }
    }
}

// MARK: - C Bridge

#[no_mangle]
pub unsafe extern "C" fn RSPGetCurrentColorCombiner(rcp: Option<&mut RCP>) -> *mut ColorCombiner {
    let rcp = rcp.unwrap();
    if let Some(cc) = rcp.rsp.color_combiner_manager.get_current_combiner() {
        cc as *mut ColorCombiner
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn RSPAddColorCombiner(
    rcp: Option<&mut RCP>,
    combiner: Option<&mut ColorCombiner>,
) {
    let rcp = rcp.unwrap();
    let combiner = combiner.unwrap();
    rcp.rsp.color_combiner_manager.add_combiner(*combiner);
}

#[no_mangle]
pub unsafe extern "C" fn RSPGetColorCombiner(
    rcp: Option<&mut RCP>,
    cc_id: u32,
) -> *mut ColorCombiner {
    let rcp = rcp.unwrap();
    if let Some(cc) = rcp.rsp.color_combiner_manager.get_combiner(cc_id) {
        cc as *mut ColorCombiner
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn RSPCreateAndInsertEmptyColorCombiner(
    rcp: Option<&mut RCP>,
    cc_id: u32,
) -> *mut ColorCombiner {
    let rcp = rcp.unwrap();

    let mut combiner = ColorCombiner::ZERO;
    combiner.cc_id = cc_id;
    rcp.rsp.color_combiner_manager.add_combiner(combiner);

    if let Some(cc) = rcp.rsp.color_combiner_manager.get_combiner(cc_id) {
        cc as *mut ColorCombiner
    } else {
        std::ptr::null_mut()
    }
}
