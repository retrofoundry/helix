use glam::{Mat4, Vec3A, Vec4, Vec4Swizzles};
use std::slice;

use super::super::{rdp::RDP, rsp::RSP};
use super::defines::{Light, Viewport, Vtx, G_MTX, G_MV, G_MW};
use super::utils::{get_cmd, get_segmented_address};
use super::{GBIDefinition, GBIResult, GBI};
use crate::extensions::glam::{FromFixedPoint, NormalizeInPlace};
use crate::graphics::rsp::{RSPGeometry, MATRIX_STACK_SIZE, MAX_LIGHTS};

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

        // TODO: Handle using tile descriptors?
        rsp.texture_scaling_factor.scale_s = scale_s;
        rsp.texture_scaling_factor.scale_t = scale_t;

        GBIResult::Continue
    }

    pub fn gsp_vertex(rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let vertex_count = get_cmd(w0, 12, 8);
        let mut write_index = get_cmd(w0, 1, 7) - get_cmd(w0, 12, 8);
        let vertices = get_segmented_address(w1) as *const Vtx;

        for i in 0..vertex_count {
            let vertex = unsafe { &(*vertices.offset(i as isize)).vertex };
            let vertex_normal = unsafe { &(*vertices.offset(i as isize)).normal };
            let staging_vertex = &mut rsp.vertex_table[write_index as usize];

            let mut x = rsp.modelview_projection_matrix.row(0).dot(Vec4::new(
                vertex.position[0] as f32,
                vertex.position[1] as f32,
                vertex.position[2] as f32,
                1.0,
            ));

            let y = rsp.modelview_projection_matrix.row(1).dot(Vec4::new(
                vertex.position[0] as f32,
                vertex.position[1] as f32,
                vertex.position[2] as f32,
                1.0,
            ));

            let z = rsp.modelview_projection_matrix.row(2).dot(Vec4::new(
                vertex.position[0] as f32,
                vertex.position[1] as f32,
                vertex.position[2] as f32,
                1.0,
            ));

            let w = rsp.modelview_projection_matrix.row(3).dot(Vec4::new(
                vertex.position[0] as f32,
                vertex.position[1] as f32,
                vertex.position[2] as f32,
                1.0,
            ));

            x = rdp.adjust_x_for_viewport(x);

            let mut U = ((vertex.texture_coords[0] as i32)
                * (rsp.texture_scaling_factor.scale_s as i32)
                >> 16) as i16;
            let mut V = ((vertex.texture_coords[1] as i32)
                * (rsp.texture_scaling_factor.scale_t as i32)
                >> 16) as i16;

            if rsp.geometry_mode & RSPGeometry::G_LIGHTING as u32 > 0 {
                if !rsp.lights_valid {
                    for i in 0..rsp.num_lights - 1 {
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
                    let mut intensity = rsp.lights_coeffs[i as usize].dot(Vec3A::new(
                        vertex_normal.normal[0] as f32,
                        vertex_normal.normal[1] as f32,
                        vertex_normal.normal[2] as f32,
                    ));

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
                    let dotx = rsp.lookat_coeffs[0 as usize].dot(Vec3A::new(
                        vertex_normal.normal[0] as f32,
                        vertex_normal.normal[1] as f32,
                        vertex_normal.normal[2] as f32,
                    ));

                    let doty = rsp.lookat_coeffs[1 as usize].dot(Vec3A::new(
                        vertex_normal.normal[0] as f32,
                        vertex_normal.normal[1] as f32,
                        vertex_normal.normal[2] as f32,
                    ));

                    U = ((dotx / 127.0 + 1.0) / 4.0) as i16
                        * rsp.texture_scaling_factor.scale_s as i16;
                    V = ((doty / 127.0 + 1.0) / 4.0) as i16
                        * rsp.texture_scaling_factor.scale_t as i16;
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

    pub fn gsp_geometry_mode(_rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let clear_bits = get_cmd(w0, 0, 24);
        let set_bits = w1;

        rsp.geometry_mode &= !clear_bits as u32;
        rsp.geometry_mode |= set_bits as u32;

        GBIResult::Continue
    }

    pub fn gsp_tri1(_rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
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

            if (rsp.geometry_mode & RSPGeometry::G_CULL_BOTH as u32)
                == RSPGeometry::G_CULL_FRONT as u32
            {
                if cross < 0.0 {
                    return GBIResult::Continue;
                }
            } else if (rsp.geometry_mode & RSPGeometry::G_CULL_BOTH as u32)
                == RSPGeometry::G_CULL_BACK as u32
            {
                if cross > 0.0 {
                    return GBIResult::Continue;
                }
            } else {
                // TODO: Safe to ignore?
                return GBIResult::Continue;
            }
        }

        // TODO: Produce draw calls for RDP to process later?
        let depth_test = rsp.geometry_mode & RSPGeometry::G_ZBUFFER as u32 == RSPGeometry::G_ZBUFFER as u32;

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

fn calculate_normal_dir(light: &Light, matrix: &Mat4, coeffs: &mut Vec3A) {
    let light_dir = Vec3A::new(
        light.dir[0] as f32 / 127.0,
        light.dir[1] as f32 / 127.0,
        light.dir[2] as f32 / 127.0,
    );

    // transpose and multiply by light dir
    coeffs[0] = matrix.col(0).xyz().dot(light_dir.into());
    coeffs[1] = matrix.col(1).xyz().dot(light_dir.into());
    coeffs[2] = matrix.col(2).xyz().dot(light_dir.into());

    coeffs.normalize_in_place();
}
