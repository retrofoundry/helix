use crate::extensions::matrix::matrix_multiply;

use super::{
    gbi::defines::Light,
    rcp::RCP,
    utils::{
        color_combiner::{ColorCombiner, ColorCombinerManager, CC, SHADER},
        texture::TextureManager,
    },
};

pub const MATRIX_STACK_SIZE: usize = 11;
const MAX_VERTICES: usize = 64;
const TEXTURE_CACHE_MAX_SIZE: usize = 500;

// excluding ambient light
pub const MAX_LIGHTS: usize = 7;

pub struct TextureScalingFactor {
    pub scale_s: u16,
    pub scale_t: u16,
}

impl TextureScalingFactor {
    pub const ZERO: Self = Self {
        scale_s: 0,
        scale_t: 0,
    };
}

#[repr(C)]
pub struct StagingVertex {
    pub position: [f32; 4],
    pub uv: [f32; 2],
    pub color: [u8; 4],
    pub clip_reject: u8,
}

impl StagingVertex {
    pub const ZERO: Self = Self {
        position: [0.0; 4],
        uv: [0.0; 2],
        color: [0; 4],
        clip_reject: 0,
    };
}

#[cfg(feature = "f3dex2")]
pub enum RSPGeometry {
    G_ZBUFFER = 1 << 0,
    G_SHADE = 1 << 2,
    G_TEXTURE_ENABLE = 0 << 0,
    G_SHADING_SMOOTH = 1 << 21,
    G_CULL_FRONT = 1 << 9,
    G_CULL_BACK = 1 << 10,
    G_CULL_BOTH = Self::G_CULL_FRONT as isize | Self::G_CULL_BACK as isize,
    G_FOG = 1 << 16,
    G_LIGHTING = 1 << 17,
    G_TEXTURE_GEN = 1 << 18,
    G_TEXTURE_GEN_LINEAR = 1 << 19,
    G_LOD = 1 << 20, /* NOT IMPLEMENTED */
    G_CLIPPING = 1 << 23,
}

pub struct RSP {
    pub geometry_mode: u32,
    pub projection_matrix: [[f32; 4]; 4],

    pub matrix_stack: [[[f32; 4]; 4]; MATRIX_STACK_SIZE],
    pub matrix_stack_pointer: usize,

    pub modelview_projection_matrix: [[f32; 4]; 4],

    pub lights_valid: bool,
    pub num_lights: u8,
    pub lights: [Light; MAX_LIGHTS + 1],

    pub fog_multiplier: i16,
    pub fog_offset: i16,

    pub texture_scaling_factor: TextureScalingFactor,

    pub vertex_table: [StagingVertex; MAX_VERTICES + 4],

    pub lights_coeffs: [[f32; 3]; MAX_LIGHTS],
    pub lookat_coeffs: [[f32; 3]; 2], // lookat_x, lookat_y

    pub texture_manager: TextureManager,
    pub color_combiner_manager: ColorCombinerManager,
}

impl RSP {
    pub fn new() -> Self {
        RSP {
            geometry_mode: 0,
            projection_matrix: [[0.0; 4]; 4],

            matrix_stack: [[[0.0; 4]; 4]; MATRIX_STACK_SIZE],
            matrix_stack_pointer: 0,

            modelview_projection_matrix: [[0.0; 4]; 4],

            lights_valid: true,
            num_lights: 0,
            lights: [Light::ZERO; MAX_LIGHTS + 1],

            fog_multiplier: 0,
            fog_offset: 0,

            texture_scaling_factor: TextureScalingFactor::ZERO,

            vertex_table: [StagingVertex::ZERO; MAX_VERTICES + 4],

            lights_coeffs: [[0.0; 3]; MAX_LIGHTS],
            lookat_coeffs: [[0.0; 3]; 2],

            texture_manager: TextureManager::new(TEXTURE_CACHE_MAX_SIZE),
            color_combiner_manager: ColorCombinerManager::new(),
        }
    }

    pub fn reset(&mut self) {
        self.matrix_stack_pointer = 1;
        self.set_num_lights(2);
    }

    pub fn recompute_mvp_matrix(&mut self) {
        self.modelview_projection_matrix
            .copy_from_slice(&matrix_multiply(
                &self.matrix_stack[self.matrix_stack_pointer - 1],
                &self.projection_matrix,
            ));
    }

    pub fn set_num_lights(&mut self, num_lights: u8) {
        self.num_lights = num_lights;
        self.lights_valid = false;
    }

    pub fn generate_color_combiner(&mut self, cc_id: u32) -> u32 {
        let mut c = [[0u8; 4]; 2];
        for i in 0..4 {
            c[0][i] = ((cc_id >> (i * 3)) & 7) as u8;
            c[1][i] = ((cc_id >> (12 + i * 3)) & 7) as u8;
        }

        let mut shader_id = (cc_id >> 24) << 24;
        let mut shader_input_mapping = [[0u8; 4]; 2];
        for i in 0..2 {
            if c[i][0] == c[i][1] || c[i][2] == CC::NIL as u8 {
                c[i][0] = 0;
                c[i][1] = 0;
                c[i][2] = 0;
            }
            let mut input_number = [0u8; 8];
            let mut next_input_number = SHADER::INPUT_1 as u8;
            for j in 0..4 {
                let mut val = 0;
                match c[i][j] {
                    x if x == CC::NIL as u8 => {}
                    x if x == CC::TEXEL0 as u8 => val = SHADER::TEXEL0 as u8,
                    x if x == CC::TEXEL1 as u8 => val = SHADER::TEXEL1 as u8,
                    x if x == CC::TEXEL0A as u8 => val = SHADER::TEXEL0A as u8,
                    x if [CC::PRIM, CC::SHADE, CC::ENV, CC::LOD]
                        .contains(&CC::from_u8(x).unwrap()) =>
                    {
                        if input_number[c[i][j] as usize] == 0 {
                            shader_input_mapping[i][(next_input_number - 1) as usize] = c[i][j];
                            input_number[c[i][j] as usize] = next_input_number;
                            next_input_number += 1;
                        }
                        val = input_number[c[i][j] as usize];
                    }
                    _ => {}
                }
                shader_id |= (val as u32) << (i * 12 + j * 3);
            }
        }

        let combiner = ColorCombiner::new(cc_id, shader_input_mapping);
        self.color_combiner_manager.add_combiner(combiner);
        shader_id
    }
}

// MARK: - C Bridge
#[no_mangle]
pub extern "C" fn RSPGenerateColorCombiner(rcp: Option<&mut RCP>, cc_id: u32) -> u32 {
    let rcp = rcp.unwrap();
    rcp.rsp.generate_color_combiner(cc_id)
}
