use std::slice;

use log::trace;

use super::super::{
    rdp::RDP,
    rsp::{RSPGeometry, MATRIX_STACK_SIZE, MAX_LIGHTS, RSP},
};
use super::defines::{Light, Viewport, Vtx, G_MTX, G_MV, G_MW};
use super::utils::{get_cmd, get_segmented_address};
use super::{GBIDefinition, GBIResult, GBI};
use crate::{
    extensions::matrix::{calculate_normal_dir, matrix_from_fixed_point, matrix_multiply},
    fast3d::{
        graphics::GraphicsContext,
        rdp::{
            OtherModeHCycleType, OtherModeH_Layout, Rect, TMEMMapEntry, G_TX_LOADTILE,
            SCREEN_HEIGHT, SCREEN_WIDTH,
        },
        rsp::MAX_VERTICES,
        utils::{
            color::R5G5B5A1,
            color_combiner::CombineParams,
            texture::{ImageSize, TextFilt, TextureImageState, TextureState},
        },
    },
};

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
        gbi.register(F3DEX2::G_VTX as usize, F3DEX2::gsp_vertex);
        gbi.register(F3DEX2::G_DL as usize, F3DEX2::sub_dl);
        gbi.register(F3DEX2::G_GEOMETRYMODE as usize, F3DEX2::gsp_geometry_mode);
        gbi.register(F3DEX2::G_TRI1 as usize, F3DEX2::gsp_tri1);
        gbi.register(F3DEX2::G_ENDDL as usize, |_, _, _, _, _| GBIResult::Return);

        gbi.register(
            F3DEX2::G_SETOTHERMODE_L as usize,
            F3DEX2::gdp_set_other_mode_l,
        );
        gbi.register(
            F3DEX2::G_SETOTHERMODE_H as usize,
            F3DEX2::gdp_set_other_mode_h,
        );
        gbi.register(F3DEX2::G_SETTIMG as usize, F3DEX2::gdp_set_texture_image);
        gbi.register(F3DEX2::G_LOADBLOCK as usize, F3DEX2::gdp_load_block);
        gbi.register(F3DEX2::G_LOADTILE as usize, F3DEX2::gdp_load_tile);
        gbi.register(F3DEX2::G_LOADTLUT as usize, F3DEX2::gdp_load_tlut);
        gbi.register(F3DEX2::G_SETTILE as usize, F3DEX2::gdp_set_tile);
        gbi.register(F3DEX2::G_SETTILESIZE as usize, F3DEX2::gdp_set_tile_size);
        gbi.register(F3DEX2::G_SETSCISSOR as usize, F3DEX2::gdp_set_scissor);
        gbi.register(F3DEX2::G_SETCOMBINE as usize, F3DEX2::gdp_set_combine);
        gbi.register(F3DEX2::G_SETENVCOLOR as usize, F3DEX2::gdp_set_env_color);
        gbi.register(F3DEX2::G_SETPRIMCOLOR as usize, F3DEX2::gdp_set_prim_color);
        gbi.register(F3DEX2::G_SETFOGCOLOR as usize, F3DEX2::gdp_set_fog_color);
        gbi.register(F3DEX2::G_SETFILLCOLOR as usize, F3DEX2::gdp_set_fill_color);
    }
}

impl F3DEX2 {
    pub fn gsp_matrix(
        _rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let params = get_cmd(w0, 0, 8) as u8 ^ G_MTX::PUSH as u8;

        let matrix: [[f32; 4]; 4];

        if cfg!(feature = "gbifloats") {
            let addr = get_segmented_address(w1) as *const f32;
            matrix = unsafe { slice::from_raw_parts(addr, 16) }
                .chunks(4)
                .map(|row| [row[0], row[1], row[2], row[3]])
                .collect::<Vec<[f32; 4]>>()
                .try_into()
                .unwrap();
        } else {
            let addr = get_segmented_address(w1) as *const i32;
            let slice = unsafe { slice::from_raw_parts(addr, 16) };
            matrix = matrix_from_fixed_point(slice);
        }

        if params & G_MTX::PROJECTION as u8 != 0 {
            if (params & G_MTX::LOAD as u8) != 0 {
                // Load the input matrix into the projection matrix
                rsp.projection_matrix.copy_from_slice(&matrix);
            } else {
                // Multiply the current projection matrix with the input matrix
                let result = matrix_multiply(&matrix, &rsp.projection_matrix);
                rsp.projection_matrix.copy_from_slice(&result);
            }
        } else {
            // Modelview matrix
            if params & G_MTX::PUSH as u8 != 0 && rsp.matrix_stack_pointer < MATRIX_STACK_SIZE {
                // Push a copy of the current matrix onto the stack
                rsp.matrix_stack_pointer += 1;

                let src_index = rsp.matrix_stack_pointer - 2;
                let dst_index = rsp.matrix_stack_pointer - 1;
                let (left, right) = rsp.matrix_stack.split_at_mut(dst_index);
                right[0].copy_from_slice(&left[src_index]);
            }

            if params & G_MTX::LOAD as u8 != 0 {
                // Load the input matrix into the current matrix
                rsp.matrix_stack[rsp.matrix_stack_pointer - 1].copy_from_slice(&matrix);
            } else {
                // Multiply the current matrix with the input matrix
                let result =
                    matrix_multiply(&matrix, &rsp.matrix_stack[rsp.matrix_stack_pointer - 1]);
                rsp.matrix_stack[rsp.matrix_stack_pointer - 1].copy_from_slice(&result);
            }

            // Clear the lights_valid flag
            rsp.lights_valid = false;
        }

        // Recalculate the modelview projection matrix
        rsp.recompute_mvp_matrix();

        GBIResult::Continue
    }

    pub fn gsp_pop_matrix(
        _rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        _w0: usize,
        w1: usize,
    ) -> GBIResult {
        let num_matrices_to_pop = w1 / 64;

        // If no matrices to pop, return
        if num_matrices_to_pop == 0 || rsp.matrix_stack_pointer == 0 {
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

        // Recalculate the modelview projection matrix
        rsp.recompute_mvp_matrix();

        GBIResult::Continue
    }

    pub fn gsp_movemem(
        rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
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
            _ => trace!("Unknown movemem index: {}", index),
        }

        GBIResult::Continue
    }

    pub fn gsp_moveword(
        _rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let index = get_cmd(w0, 16, 8) as u8;
        let _offset: u16 = get_cmd(w0, 0, 16) as u16;

        match index {
            index if index == G_MW::NUMLIGHT as u8 => rsp.set_num_lights(w1 as u8 / 24 + 1),
            index if index == G_MW::FOG as u8 => {
                rsp.fog_multiplier = (w1 >> 16) as i16;
                rsp.fog_offset = w1 as i16;
            }
            // TODO: HANDLE G_MW_SEGMENT
            _ => {}
        }

        GBIResult::Continue
    }

    pub fn gsp_texture(
        rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let scale_s = get_cmd(w1, 16, 16) as u16;
        let scale_t = get_cmd(w1, 0, 16) as u16;
        let level = get_cmd(w0, 11, 3) as u8;
        let tile = get_cmd(w0, 8, 3) as u8;
        let on = get_cmd(w0, 1, 7) as u8;

        if rdp.texture_state.tile != tile {
            rdp.textures_changed[0] = true;
            rdp.textures_changed[1] = true;
        }

        rdp.texture_state = TextureState::new(on != 0, tile, level, scale_s, scale_t);

        GBIResult::Continue
    }

    pub fn gsp_vertex(
        rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let vertex_count = get_cmd(w0, 12, 8) as u8;
        let mut write_index = get_cmd(w0, 1, 7) as u8 - get_cmd(w0, 12, 8) as u8;
        let vertices = get_segmented_address(w1) as *const Vtx;

        for i in 0..vertex_count {
            let vertex = unsafe { &(*vertices.offset(i as isize)).vertex };
            let vertex_normal = unsafe { &(*vertices.offset(i as isize)).normal };
            let staging_vertex = &mut rsp.vertex_table[write_index as usize];

            let mut x = vertex.position[0] as f32 * rsp.modelview_projection_matrix[0][0]
                + vertex.position[1] as f32 * rsp.modelview_projection_matrix[1][0]
                + vertex.position[2] as f32 * rsp.modelview_projection_matrix[2][0]
                + rsp.modelview_projection_matrix[3][0];

            let y = vertex.position[0] as f32 * rsp.modelview_projection_matrix[0][1]
                + vertex.position[1] as f32 * rsp.modelview_projection_matrix[1][1]
                + vertex.position[2] as f32 * rsp.modelview_projection_matrix[2][1]
                + rsp.modelview_projection_matrix[3][1];

            let z = vertex.position[0] as f32 * rsp.modelview_projection_matrix[0][2]
                + vertex.position[1] as f32 * rsp.modelview_projection_matrix[1][2]
                + vertex.position[2] as f32 * rsp.modelview_projection_matrix[2][2]
                + rsp.modelview_projection_matrix[3][2];

            let w = vertex.position[0] as f32 * rsp.modelview_projection_matrix[0][3]
                + vertex.position[1] as f32 * rsp.modelview_projection_matrix[1][3]
                + vertex.position[2] as f32 * rsp.modelview_projection_matrix[2][3]
                + rsp.modelview_projection_matrix[3][3];

            x = rdp.adjust_x_for_viewport(x);

            let mut U = ((vertex.texture_coords[0] as i32) * (rdp.texture_state.scale_s as i32)
                >> 16) as i16;
            let mut V = ((vertex.texture_coords[1] as i32) * (rdp.texture_state.scale_t as i32)
                >> 16) as i16;

            if rsp.geometry_mode & RSPGeometry::G_LIGHTING as u32 > 0 {
                if !rsp.lights_valid {
                    for i in 0..rsp.num_lights {
                        calculate_normal_dir(
                            &rsp.lights[i as usize],
                            &rsp.matrix_stack[rsp.matrix_stack_pointer as usize - 1],
                            &mut rsp.lights_coeffs[i as usize],
                        );
                    }

                    static LOOKAT_X: Light = Light::new([0, 0, 0], [0, 0, 0], [127, 0, 0]);
                    static LOOKAT_Y: Light = Light::new([0, 0, 0], [0, 0, 0], [0, 127, 0]);

                    calculate_normal_dir(
                        &LOOKAT_X,
                        &rsp.matrix_stack[rsp.matrix_stack_pointer as usize - 1],
                        &mut rsp.lookat_coeffs[0],
                    );

                    calculate_normal_dir(
                        &LOOKAT_Y,
                        &rsp.matrix_stack[rsp.matrix_stack_pointer as usize - 1],
                        &mut rsp.lookat_coeffs[1],
                    );

                    rsp.lights_valid = true
                }

                let mut r = rsp.lights[rsp.num_lights as usize - 1].col[0] as f32;
                let mut g = rsp.lights[rsp.num_lights as usize - 1].col[1] as f32;
                let mut b = rsp.lights[rsp.num_lights as usize - 1].col[2] as f32;

                for i in 0..rsp.num_lights - 1 {
                    let mut intensity = vertex_normal.normal[0] as f32
                        * rsp.lights_coeffs[i as usize][0]
                        + vertex_normal.normal[1] as f32 * rsp.lights_coeffs[i as usize][1]
                        + vertex_normal.normal[2] as f32 * rsp.lights_coeffs[i as usize][2];

                    intensity /= 127.0;

                    if intensity > 0.0 {
                        r += intensity * rsp.lights[i as usize].col[0] as f32;
                        g += intensity * rsp.lights[i as usize].col[1] as f32;
                        b += intensity * rsp.lights[i as usize].col[2] as f32;
                    }
                }

                staging_vertex.color[0] = if r > 255.0 { 255 } else { r as u8 };
                staging_vertex.color[1] = if g > 255.0 { 255 } else { g as u8 };
                staging_vertex.color[2] = if b > 255.0 { 255 } else { b as u8 };

                if rsp.geometry_mode & RSPGeometry::G_TEXTURE_GEN as u32 > 0 {
                    let dotx = vertex_normal.normal[0] as f32 * rsp.lookat_coeffs[0][0]
                        + vertex_normal.normal[1] as f32 * rsp.lookat_coeffs[0][1]
                        + vertex_normal.normal[2] as f32 * rsp.lookat_coeffs[0][2];

                    let doty = vertex_normal.normal[0] as f32 * rsp.lookat_coeffs[1][0]
                        + vertex_normal.normal[1] as f32 * rsp.lookat_coeffs[1][1]
                        + vertex_normal.normal[2] as f32 * rsp.lookat_coeffs[1][2];

                    U = ((dotx / 127.0 + 1.0) / 4.0) as i16 * rdp.texture_state.scale_s as i16;
                    V = ((doty / 127.0 + 1.0) / 4.0) as i16 * rdp.texture_state.scale_t as i16;
                }
            } else {
                staging_vertex.color[0] = vertex.color[0];
                staging_vertex.color[1] = vertex.color[1];
                staging_vertex.color[2] = vertex.color[2];
            }

            staging_vertex.uv[0] = U as f32;
            staging_vertex.uv[1] = V as f32;

            // trivial clip rejection
            staging_vertex.clip_reject = 0;
            if x < -w {
                staging_vertex.clip_reject |= 1;
            }
            if x > w {
                staging_vertex.clip_reject |= 2;
            }
            if y < -w {
                staging_vertex.clip_reject |= 4;
            }
            if y > w {
                staging_vertex.clip_reject |= 8;
            }
            if z < -w {
                staging_vertex.clip_reject |= 16;
            }
            if z > w {
                staging_vertex.clip_reject |= 32;
            }

            staging_vertex.position[0] = x;
            staging_vertex.position[1] = y;
            staging_vertex.position[2] = z;
            staging_vertex.position[3] = w;

            if rsp.geometry_mode & RSPGeometry::G_FOG as u32 > 0 {
                let w = if w.abs() < 0.001 { 0.001 } else { w };

                let winv = 1.0 / w;
                let winv = if winv < 0.0 { 32767.0 } else { winv };

                let fog = z * winv * rsp.fog_multiplier as f32 + rsp.fog_offset as f32;
                let fog = if fog < 0.0 { 0.0 } else { fog };
                let fog = if fog > 255.0 { 255.0 } else { fog };

                staging_vertex.color[3] = fog as u8;
            } else {
                staging_vertex.color[3] = vertex.color[3];
            }

            write_index += 1;
        }

        GBIResult::Continue
    }

    pub fn gsp_geometry_mode(
        _rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let clear_bits = get_cmd(w0, 0, 24);
        let set_bits = w1;

        rsp.geometry_mode &= clear_bits as u32;
        rsp.geometry_mode |= set_bits as u32;

        GBIResult::Continue
    }

    pub fn gsp_tri1(
        rdp: &mut RDP,
        rsp: &mut RSP,
        gfx_context: &GraphicsContext,
        w0: usize,
        _w1: usize,
    ) -> GBIResult {
        let vertex_id1 = get_cmd(w0, 16, 8) / 2;
        let vertex_id2 = get_cmd(w0, 8, 8) / 2;
        let vertex_id3 = get_cmd(w0, 0, 8) / 2;

        let vertex1 = &rsp.vertex_table[vertex_id1];
        let vertex2 = &rsp.vertex_table[vertex_id2];
        let vertex3 = &rsp.vertex_table[vertex_id3];
        let vertex_array = [vertex1, vertex2, vertex3];

        if (vertex1.clip_reject & vertex2.clip_reject & vertex3.clip_reject) > 0 {
            // ...whole tri is offscreen, cull.
            return GBIResult::Continue;
        }

        if (rsp.geometry_mode & RSPGeometry::G_CULL_BOTH as u32) > 0 {
            let dx1 = vertex1.position[0] / vertex1.position[3]
                - vertex2.position[0] / vertex2.position[3];
            let dy1 = vertex1.position[1] / vertex1.position[3]
                - vertex2.position[1] / vertex2.position[3];
            let dx2 = vertex3.position[0] / vertex3.position[3]
                - vertex2.position[0] / vertex2.position[3];
            let dy2 = vertex3.position[1] / vertex3.position[3]
                - vertex2.position[1] / vertex2.position[3];
            let mut cross = dx1 * dy2 - dy1 * dx2;

            // If any verts are past any clipping plane..
            if (vertex1.position[3] < 0.0)
                ^ (vertex2.position[3] < 0.0)
                ^ (vertex3.position[3] < 0.0)
            {
                // If one vertex lies behind the eye, negating cross will give the correct result.
                // If all vertices lie behind the eye, the triangle will be rejected anyway.
                cross = -cross;
            }

            match rsp.geometry_mode & RSPGeometry::G_CULL_BOTH as u32 {
                geometry_mode if geometry_mode == RSPGeometry::G_CULL_FRONT as u32 => {
                    if cross <= 0.0 {
                        return GBIResult::Continue;
                    }
                }
                geometry_mode if geometry_mode == RSPGeometry::G_CULL_BACK as u32 => {
                    if cross >= 0.0 {
                        return GBIResult::Continue;
                    }
                }
                geometry_mode if geometry_mode == RSPGeometry::G_CULL_BOTH as u32 => {
                    return GBIResult::Continue;
                }
                _ => {}
            }
        }

        // TODO: Produce draw calls for RDP to process later?
        rdp.update_render_state(gfx_context, rsp.geometry_mode, &vertex_array);

        GBIResult::Continue
    }

    pub fn sub_dl(
        _rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        if get_cmd(w0, 16, 1) == 0 {
            // Push return address
            let new_addr = get_segmented_address(w1);
            return GBIResult::Recurse(new_addr);
        } else {
            let new_addr = get_segmented_address(w1);
            return GBIResult::SetAddress(new_addr);
        }
    }

    pub fn gdp_set_other_mode_l(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let shift = 31 - get_cmd(w0, 8, 8) - get_cmd(w0, 0, 8);
        let mask = get_cmd(w0, 0, 8) + 1;
        let mode = w1;

        F3DEX2::gdp_other_mode(rdp, shift, mask, mode as u64)
    }

    pub fn gdp_set_other_mode_h(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let shift = 63 - get_cmd(w0, 8, 8) - get_cmd(w0, 0, 8);
        let mask = get_cmd(w0, 0, 8) + 1;
        let mode = (w1 as u64) << 32;

        F3DEX2::gdp_other_mode(rdp, shift, mask, mode)
    }

    pub fn gdp_other_mode(rdp: &mut RDP, shift: usize, mask: usize, mode: u64) -> GBIResult {
        let mask = (((1 as u64) << (mask as u64)) - 1) << shift as u64;
        let mut om = rdp.other_mode_l as u64 | ((rdp.other_mode_h as u64) << 32);
        om = (om & !mask) | mode as u64;

        rdp.other_mode_l = om as u32;
        rdp.other_mode_h = (om >> 32) as u32;

        GBIResult::Continue
    }

    // gdp_set_scissor
    pub fn gdp_set_scissor(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let _mode = get_cmd(w1, 24, 2);
        let ulx = get_cmd(w0, 12, 12);
        let uly = get_cmd(w0, 0, 12);
        let lrx = get_cmd(w1, 12, 12);
        let lry = get_cmd(w1, 0, 12);

        let x = ulx as f32 / 4.0 * rdp.scaled_x();
        let y = (SCREEN_HEIGHT - lry as f32 / 4.0) * rdp.scaled_y();
        let width = (lrx as f32 - ulx as f32) / 4.0 * rdp.scaled_x();
        let height = (lry as f32 - uly as f32) / 4.0 * rdp.scaled_y();

        rdp.scissor.x = x as u16;
        rdp.scissor.y = y as u16;
        rdp.scissor.width = width as u16;
        rdp.scissor.height = height as u16;

        rdp.viewport_or_scissor_changed = true;
        GBIResult::Continue
    }

    pub fn gdp_set_combine(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        rdp.combine = CombineParams::decode(w0, w1);

        GBIResult::Continue
    }

    pub fn gdp_set_tile(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let format = get_cmd(w0, 21, 3) as u8;
        let size = get_cmd(w0, 19, 2) as u8;
        let line = get_cmd(w0, 9, 9) as u16;
        let tmem = get_cmd(w0, 0, 9) as u16;
        let tile = get_cmd(w1, 24, 3) as u8;
        let palette = get_cmd(w1, 20, 4) as u8;
        let cm_t: u8 = get_cmd(w1, 18, 2) as u8;
        let mask_t: u8 = get_cmd(w1, 14, 4) as u8;
        let shift_t: u8 = get_cmd(w1, 10, 4) as u8;
        let cm_s: u8 = get_cmd(w1, 8, 2) as u8;
        let mask_s: u8 = get_cmd(w1, 4, 4) as u8;
        let shift_s: u8 = get_cmd(w1, 0, 4) as u8;

        let tile = &mut rdp.tile_descriptors[tile as usize];
        tile.set_format(format);
        tile.set_size(size);
        tile.line = line;
        tile.tmem = tmem;
        tile.palette = palette;
        tile.cm_t = cm_t;
        tile.mask_t = mask_t;
        tile.shift_t = shift_t;
        tile.cm_s = cm_s;
        tile.mask_s = mask_s;
        tile.shift_s = shift_s;

        rdp.textures_changed[0] = true;
        rdp.textures_changed[1] = true;

        GBIResult::Continue
    }

    pub fn gdp_set_tile_size(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let tile = get_cmd(w1, 24, 3) as u8;
        let uls = get_cmd(w0, 12, 12) as u16;
        let ult = get_cmd(w0, 0, 12) as u16;
        let lrs = get_cmd(w1, 12, 12) as u16;
        let lrt = get_cmd(w1, 0, 12) as u16;

        let tile = &mut rdp.tile_descriptors[tile as usize];
        tile.uls = uls;
        tile.ult = ult;
        tile.lrs = lrs;
        tile.lrt = lrt;

        rdp.textures_changed[0] = true;
        rdp.textures_changed[1] = true;

        GBIResult::Continue
    }

    pub fn gdp_set_texture_image(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let format = get_cmd(w0, 21, 3) as u8;
        let size = get_cmd(w0, 19, 2) as u8;
        let width = get_cmd(w0, 0, 10) as u16;
        let address = get_segmented_address(w1);

        rdp.texture_image_state = TextureImageState {
            format,
            size,
            width,
            address,
        };

        GBIResult::Continue
    }

    pub fn gdp_load_tlut(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        _w0: usize,
        w1: usize,
    ) -> GBIResult {
        let tile = get_cmd(w1, 24, 3);
        let high_index = get_cmd(w1, 14, 10) as u16;

        assert!(tile == G_TX_LOADTILE);
        assert!(rdp.texture_image_state.size == ImageSize::G_IM_SIZ_16b as u8); // TLUTs are always 16-bit (so far)
        assert!(
            rdp.tile_descriptors[tile as usize].tmem == 256
                && (high_index <= 127 || high_index == 255)
                || rdp.tile_descriptors[tile as usize].tmem == 384 && high_index == 127
        );

        trace!("gdp_load_tlut(tile: {}, high_index: {})", tile, high_index);

        let tile = &mut rdp.tile_descriptors[tile as usize];
        rdp.tmem_map.insert(
            tile.tmem,
            TMEMMapEntry::new(rdp.texture_image_state.address),
        );

        GBIResult::Continue
    }

    pub fn gdp_load_block(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        let tile = get_cmd(w1, 24, 3);
        let uls = get_cmd(w0, 12, 12);
        let ult = get_cmd(w0, 0, 12);
        let _texels = get_cmd(w1, 12, 12) as u16;
        let _dxt = get_cmd(w1, 0, 12);

        // First, verify that we're loading the whole texture.
        assert!(uls == 0 && ult == 0);
        // Verify that we're loading into LOADTILE.
        assert!(tile == G_TX_LOADTILE);

        let tile = &mut rdp.tile_descriptors[tile as usize];
        rdp.tmem_map.insert(
            tile.tmem,
            TMEMMapEntry::new(rdp.texture_image_state.address),
        );

        trace!("texture {} is being marked as has changed", tile.tmem / 256);
        let tmem_index = if tile.tmem != 0 { 1 } else { 0 };
        rdp.textures_changed[tmem_index as usize] = true;

        GBIResult::Continue
    }

    pub fn gdp_load_tile(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        trace!("gdp_load_tile(w0: {}, w1: {})", w0, w1);
        let tile_index = get_cmd(w1, 24, 3);
        let uls = get_cmd(w0, 12, 12) as u16;
        let ult = get_cmd(w0, 0, 12) as u16;
        let lrs = get_cmd(w1, 12, 12) as u16;
        let lrt = get_cmd(w1, 0, 12) as u16;

        // First, verify that we're loading the whole texture.
        assert!(uls == 0 && ult == 0);
        // Verify that we're loading into LOADTILE.
        assert!(tile_index == G_TX_LOADTILE);

        let tile = &mut rdp.tile_descriptors[tile_index as usize];
        rdp.tmem_map.insert(
            tile.tmem,
            TMEMMapEntry::new(rdp.texture_image_state.address),
        );

        // TODO: Really necessary?
        tile.uls = uls;
        tile.ult = ult;
        tile.lrs = lrs;
        tile.lrt = lrt;

        trace!("texture {} is being marked as has changed", tile.tmem / 256);
        let tmem_index = if tile.tmem != 0 { 1 } else { 0 };
        rdp.textures_changed[tmem_index as usize] = true;

        GBIResult::Continue
    }

    pub fn gdp_set_env_color(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        _w0: usize,
        w1: usize,
    ) -> GBIResult {
        let r = get_cmd(w1, 24, 8) as u8;
        let g = get_cmd(w1, 16, 8) as u8;
        let b = get_cmd(w1, 8, 8) as u8;
        let a = get_cmd(w1, 0, 8) as u8;

        rdp.env_color = [r, g, b, a];
        GBIResult::Continue
    }

    pub fn gdp_set_prim_color(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        _w0: usize,
        w1: usize,
    ) -> GBIResult {
        let r = get_cmd(w1, 24, 8) as u8;
        let g = get_cmd(w1, 16, 8) as u8;
        let b = get_cmd(w1, 8, 8) as u8;
        let a = get_cmd(w1, 0, 8) as u8;

        rdp.prim_color = [r, g, b, a];
        GBIResult::Continue
    }

    pub fn gdp_set_fog_color(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        _w0: usize,
        w1: usize,
    ) -> GBIResult {
        let r = get_cmd(w1, 24, 8) as u8;
        let g = get_cmd(w1, 16, 8) as u8;
        let b = get_cmd(w1, 8, 8) as u8;
        let a = get_cmd(w1, 0, 8) as u8;

        rdp.fog_color = [r, g, b, a];
        GBIResult::Continue
    }

    pub fn gdp_set_fill_color(
        rdp: &mut RDP,
        _rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        _w0: usize,
        w1: usize,
    ) -> GBIResult {
        let packed_color = w1 as u16;
        let color = R5G5B5A1::to_rgba(packed_color);

        rdp.fill_color = [color[0], color[1], color[2], color[3]];
        GBIResult::Continue
    }

    pub fn draw_rectangle(rdp: &mut RDP, rsp: &mut RSP, ulx: i32, uly: i32, lrx: i32, lry: i32) {
        let saved_other_mode_h = rdp.other_mode_h;
        let cycle_type = RDP::get_cycle_type_from_other_mode_h(rdp.other_mode_h);

        if cycle_type == OtherModeHCycleType::G_CYC_COPY {
            rdp.other_mode_h = (rdp.other_mode_h
                & !(3 << OtherModeH_Layout::G_MDSFT_TEXTFILT as u32))
                | (TextFilt::G_TF_POINT as u32);
        }

        // U10.2 coordinates
        let mut ulxf = ulx as f32 / (4.0 * (SCREEN_WIDTH / 2.0)) - 1.0;
        let ulyf = -(uly as f32 / (4.0 * (SCREEN_HEIGHT / 2.0))) + 1.0;
        let mut lrxf = lrx as f32 / (4.0 * (SCREEN_WIDTH / 2.0)) - 1.0;
        let lryf = -(lry as f32 / (4.0 * (SCREEN_HEIGHT / 2.0))) + 1.0;

        ulxf = rdp.adjust_x_for_viewport(ulxf);
        lrxf = rdp.adjust_x_for_viewport(lrxf);

        {
            let ul = &mut rsp.vertex_table[MAX_VERTICES + 0];
            ul.position[0] = ulxf;
            ul.position[1] = ulyf;
            ul.position[2] = -1.0;
            ul.position[3] = 1.0;
        }

        {
            let ll = &mut rsp.vertex_table[MAX_VERTICES + 1];
            ll.position[0] = ulxf;
            ll.position[1] = lryf;
            ll.position[2] = -1.0;
            ll.position[3] = 1.0;
        }

        {
            let lr = &mut rsp.vertex_table[MAX_VERTICES + 2];
            lr.position[0] = lrxf;
            lr.position[1] = lryf;
            lr.position[2] = -1.0;
            lr.position[3] = 1.0;
        }

        {
            let ur = &mut rsp.vertex_table[MAX_VERTICES + 3];
            ur.position[0] = lrxf;
            ur.position[1] = ulyf;
            ur.position[2] = -1.0;
            ur.position[3] = 1.0;
        }

        // The coordinates for texture rectangle shall bypass the viewport setting
        let default_viewport = Rect::new(
            0,
            0,
            rdp.output_dimensions.width as u16,
            rdp.output_dimensions.height as u16,
        );
        let viewport_saved = rdp.viewport;
        let geometry_mode_saved = rsp.geometry_mode;

        rdp.viewport = default_viewport;
        rdp.viewport_or_scissor_changed = true;
        rsp.geometry_mode = 0;

        // TODO: call sp_tri1
        // TODO: call sp_tri1

        rsp.geometry_mode = geometry_mode_saved;
        rdp.viewport = viewport_saved;
        rdp.viewport_or_scissor_changed = true;

        if cycle_type == OtherModeHCycleType::G_CYC_COPY {
            rdp.other_mode_h = saved_other_mode_h;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::F3DEX2;
    use crate::fast3d::{
        graphics::{DummyGraphicsDevice, GraphicsContext},
        rdp::RDP,
        rsp::RSP,
    };

    #[test]
    fn test_moveword() {
        // NUM_LIGHT
        let w0: usize = 3674341376;
        let w1: usize = 24;

        let mut rsp = RSP::new();
        let mut rdp = RDP::new();

        let dummy_gfx_context = GraphicsContext::new(Box::new(DummyGraphicsDevice {}));
        F3DEX2::gsp_moveword(&mut rdp, &mut rsp, &dummy_gfx_context, w0, w1);
        assert!(rsp.num_lights == 2);

        // FOG
        let w0: usize = 3674734592;
        let w1: usize = 279638102;

        let mut rsp = RSP::new();
        let mut rdp = RDP::new();

        let dummy_gfx_context = GraphicsContext::new(Box::new(DummyGraphicsDevice {}));
        F3DEX2::gsp_moveword(&mut rdp, &mut rsp, &dummy_gfx_context, w0, w1);
        assert!(rsp.fog_multiplier == 4266);
        assert!(rsp.fog_offset == -4010);
    }
}
