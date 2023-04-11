use glam::Mat4;
use std::slice;

use super::super::{rdp::RDP, rsp::RSP};
use super::defines::{Light, Viewport, G_MTX, G_MV, G_MW};
use super::utils::{get_cmd, get_segmented_address};
use super::{GBIDefinition, GBIResult, GBI};
use crate::extensions::glam::FromFixedPoint;
use crate::graphics::rsp::{MATRIX_STACK_SIZE, MAX_LIGHTS};

pub enum F3DEX2 {
    // DMA
    G_VTX = 0x01,
    G_MODIFYVTX = 0x02,
    G_CULLDL = 0x03,
    G_BRANCH_Z = 0x04,
    G_TRI1 = 0x05,
    G_TRI2 = 0x06,
    G_QUAD = 0x07,
    G_LINE3D = 0x08,

    // RSP
    G_TEXTURE = 0xD7,
    G_POPMTX = 0xD8,
    G_GEOMETRYMODE = 0xD9,
    G_MTX = 0xDA,
    G_MOVEWORD = 0xDB,
    G_MOVEMEM = 0xDC,
    G_LOAD_UCODE = 0xDD,
    G_DL = 0xDE,
    G_ENDDL = 0xDF,

    // RDP
    G_SETCIMG = 0xFF,
    G_SETZIMG = 0xFE,
    G_SETTIMG = 0xFD,
    G_SETCOMBINE = 0xFC,
    G_SETENVCOLOR = 0xFB,
    G_SETPRIMCOLOR = 0xFA,
    G_SETBLENDCOLOR = 0xF9,
    G_SETFOGCOLOR = 0xF8,
    G_SETFILLCOLOR = 0xF7,
    G_FILLRECT = 0xF6,
    G_SETTILE = 0xF5,
    G_LOADTILE = 0xF4,
    G_LOADBLOCK = 0xF3,
    G_SETTILESIZE = 0xF2,
    G_LOADTLUT = 0xF0,
    G_RDPSETOTHERMODE = 0xEF,
    G_SETPRIMDEPTH = 0xEE,
    G_SETSCISSOR = 0xED,
    G_SETCONVERT = 0xEC,
    G_SETKEYR = 0xEB,
    G_SETKEYFB = 0xEA,
    G_RDPFULLSYNC = 0xE9,
    G_RDPTILESYNC = 0xE8,
    G_RDPPIPESYNC = 0xE7,
    G_RDPLOADSYNC = 0xE6,
    G_TEXRECTFLIP = 0xE5,
    G_TEXRECT = 0xE4,
    G_SETOTHERMODE_H = 0xE3,
    G_SETOTHERMODE_L = 0xE2,
}

impl GBIDefinition for F3DEX2 {
    fn setup(gbi: &mut GBI) {
        gbi.register(F3DEX2::G_MTX as usize, F3DEX2::gsp_matrix);
        gbi.register(F3DEX2::G_POPMTX as usize, F3DEX2::gsp_pop_matrix);
        gbi.register(F3DEX2::G_MOVEMEM as usize, F3DEX2::gsp_movemem);
        gbi.register(F3DEX2::G_MOVEWORD as usize, F3DEX2::gsp_moveword);
        gbi.register(F3DEX2::G_TEXTURE as usize, F3DEX2::gsp_texture);
        gbi.register(F3DEX2::G_GEOMETRYMODE as usize, F3DEX2::gsp_geometry_mode);
        gbi.register(F3DEX2::G_DL as usize, F3DEX2::sub_dl);
        gbi.register(F3DEX2::G_ENDDL as usize, |_, _, _, _| GBIResult::Return);
    }
}

impl F3DEX2 {
    pub fn gsp_matrix(_rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let params = get_cmd(w0, 0, 8) ^ G_MTX::PUSH as usize;

        let addr = get_segmented_address(w1) as *const i32;
        let slice = unsafe { slice::from_raw_parts(addr, 16) };
        let matrix = Mat4::from_fixed_point(slice);

        if params & G_MTX::PROJECTION as usize != 0 {
            if (params & G_MTX::LOAD as usize) != 0 {
                // Load the input matrix into the projection matrix
                rsp.projection_matrix = matrix;
            } else {
                // Multiply the current projection matrix with the input matrix
                rsp.projection_matrix *= matrix;
            }
        } else {
            // Modelview matrix
            if params & G_MTX::PUSH as usize != 0 && rsp.matrix_stack_pointer < MATRIX_STACK_SIZE {
                // Push a copy of the current matrix onto the stack
                rsp.matrix_stack_pointer += 1;
                let source = rsp.matrix_stack[rsp.matrix_stack_pointer - 2].clone();
                rsp.matrix_stack[rsp.matrix_stack_pointer - 1].clone_from(&source);
            }

            if params & G_MTX::LOAD as usize != 0 {
                // Load the input matrix into the current matrix
                rsp.matrix_stack[rsp.matrix_stack_pointer - 1].clone_from(&matrix);
            } else {
                // Multiply the current matrix with the input matrix
                rsp.matrix_stack[rsp.matrix_stack_pointer - 1] *= matrix;
            }

            // Clear the MVP light valid flag
            rsp.clear_mvp_light_valid();
        }

        // Recalculate the modelview projection matrix
        rsp.calculate_mvp_matrix();

        GBIResult::Continue
    }

    pub fn gsp_pop_matrix(_rdp: &mut RDP, rsp: &mut RSP, _w0: usize, w1: usize) -> GBIResult {
        // Calculate the number of matrices to pop
        let num_matrices_to_pop = w1 / 64;

        // If no matrices to pop, return
        if num_matrices_to_pop == 0 {
            return GBIResult::Continue;
        }

        // Pop the specified number of matrices
        for _ in 0..num_matrices_to_pop {
            // Check if there are matrices left to pop
            if rsp.matrix_stack_pointer > 0 {
                // Decrement the matrix stack index
                rsp.matrix_stack_pointer -= 1;
            }
        }

        // Clear the MVP and light valid flag
        rsp.clear_mvp_light_valid();

        // Recalculate the modelview projection matrix
        rsp.calculate_mvp_matrix();

        GBIResult::Continue
    }

    pub fn gsp_movemem(rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let index: u8 = get_cmd(w0, 0, 8) as u8;
        let offset: u8 = get_cmd(w0, 8, 8) as u8 * 8;
        let data = get_segmented_address(w1);

        match index {
            index if index == G_MV::VIEWPORT as u8 => {
                let viewport_ptr = data as *const Viewport;
                let viewport = unsafe { &*viewport_ptr };
                rdp.calculate_and_set_viewport(*viewport);
            }
            index if index == G_MV::LIGHT as u8 => {
                let light_index = (offset as i8 / 24) - 2;
                if light_index >= 0 && (light_index as usize) < MAX_LIGHTS {
                    let light_ptr = data as *const Light;
                    let light = unsafe { &*light_ptr };
                    rsp.lights[light_index as usize] = *light;
                }
            }
            // TODO: HANDLE G_MV_LOOKATY & G_MV_LOOKATX
            _ => println!("Unknown movemem index: {}", index),
        }

        GBIResult::Continue
    }

    pub fn gsp_moveword(_rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let index = get_cmd(w0, 16, 8) as u8;
        let _offset: u16 = get_cmd(w0, 0, 16) as u16;
        let data = get_segmented_address(w1);

        match index {
            index if index == G_MW::NUMLIGHT as u8 => rsp.set_num_lights((data / 24 + 1) as u8),
            index if index == G_MW::FOG as u8 => {
                rsp.fog_multiplier = (data >> 16) as i16;
                rsp.fog_offset = data as i16;
            }
            // TODO: HANDLE G_MW_SEGMENT
            _ => println!("Unknown movemem index: {}", index),
        }

        GBIResult::Continue
    }

    pub fn gsp_texture(_rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let scale_s = get_cmd(w1, 16, 16) as u16;
        let scale_t = get_cmd(w1, 0, 16) as u16;
        let _level = get_cmd(w0, 11, 3) as u8;
        let _tile = get_cmd(w0, 8, 3) as u8;
        let _on = get_cmd(w0, 1, 7) as u8;

        rsp.texture_scaling_factor.scale_s = scale_s;
        rsp.texture_scaling_factor.scale_t = scale_t;

        GBIResult::Continue
    }

    pub fn gsp_geometry_mode(_rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let clear_bits = get_cmd(w0, 0, 24);
        let set_bits = w1;

        rsp.geometry_mode &= !clear_bits as u32;
        rsp.geometry_mode |= set_bits as u32;

        GBIResult::Continue
    }

    pub fn sub_dl(_rdp: &mut RDP, _rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        if get_cmd(w0, 16, 1) == 0 {
            // Push return address
            let new_addr = get_segmented_address(w1);
            return GBIResult::Recurse(new_addr);
        } else {
            let new_addr = get_segmented_address(w1);
            return GBIResult::SetAddress(new_addr);
        }
    }
}
