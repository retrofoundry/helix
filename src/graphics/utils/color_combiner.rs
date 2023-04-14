use std::collections::HashMap;

use crate::graphics::{gfx_device::ShaderProgram, rcp::RCP};

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
            println!("[ColorCombinerManager] Found combiner with id {}", cc_id);
            Some(cc)
        } else {
            println!("[ColorCombinerManager] No combiner with id {}", cc_id);
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
    pub fn new() -> Self {
        Self {
            cc_id: 0,
            prg: std::ptr::null_mut(),
            shader_input_mapping: [[0; 4]; 2],
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

    let mut combiner = ColorCombiner::new();
    combiner.cc_id = cc_id;
    rcp.rsp.color_combiner_manager.add_combiner(combiner);

    if let Some(cc) = rcp.rsp.color_combiner_manager.get_combiner(cc_id) {
        cc as *mut ColorCombiner
    } else {
        std::ptr::null_mut()
    }
}
