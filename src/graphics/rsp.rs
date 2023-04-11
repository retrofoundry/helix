use glam::{Mat4, Vec3A, Vec4};

use super::gbi::defines::Light;

pub const MATRIX_STACK_SIZE: usize = 11;
const MAX_VERTICES: usize = 64;

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

pub struct StagingVertex {
    pub position: Vec4,
    pub uv: [f32; 2],
    pub color: [u8; 4],
    pub clip_reject: u8,
}

impl StagingVertex {
    pub const ZERO: Self = Self {
        position: Vec4::ZERO,
        uv: [0.0; 2],
        color: [0; 4],
        clip_reject: 0,
    };
}

pub enum RSPGeometry {
    G_ZBUFFER = 1 << 0,
    G_SHADE = 1 << 2,
    G_CULL_FRONT = 1 << 9,
    G_CULL_BACK = 1 << 10,
    G_CULL_BOTH = 1 << 9 | 1 << 10,
    G_FOG = 1 << 16,
    G_LIGHTING = 1 << 17,
    G_TEXTURE_GEN = 1 << 18,
    G_TEXTURE_GEN_LINEAR = 1 << 19,
    G_SHADING_SMOOTH = 1 << 21,
    G_CLIPPING = 1 << 23,
}

pub struct RSP {
    pub geometry_mode: u32,
    pub projection_matrix: Mat4,

    pub matrix_stack: [Mat4; MATRIX_STACK_SIZE],
    pub matrix_stack_pointer: usize,

    pub mvp_valid: bool,
    pub modelview_projection_matrix: Mat4,

    pub lights_valid: bool,
    pub num_lights: u8,
    pub lights: [Light; MAX_LIGHTS + 1],

    pub fog_multiplier: i16,
    pub fog_offset: i16,

    pub texture_scaling_factor: TextureScalingFactor,

    pub vertex_table: [StagingVertex; MAX_VERTICES],

    pub lights_coeffs: [Vec3A; MAX_LIGHTS],
    pub lookat_coeffs: [Vec3A; 2], // lookat_x, lookat_y
}

impl RSP {
    pub fn new() -> Self {
        RSP {
            geometry_mode: 0,
            projection_matrix: Mat4::ZERO,

            matrix_stack: [Mat4::ZERO; MATRIX_STACK_SIZE],
            matrix_stack_pointer: 0,

            mvp_valid: true,
            modelview_projection_matrix: Mat4::ZERO,

            lights_valid: true,
            num_lights: 0,
            lights: [Light::ZERO; MAX_LIGHTS + 1],

            fog_multiplier: 0,
            fog_offset: 0,

            texture_scaling_factor: TextureScalingFactor::ZERO,

            vertex_table: [StagingVertex::ZERO; MAX_VERTICES],

            lights_coeffs: [Vec3A::ZERO; MAX_LIGHTS],
            lookat_coeffs: [Vec3A::ZERO; 2],
        }
    }

    pub fn reset(&mut self) {
        self.matrix_stack_pointer = 1;
        self.set_num_lights(2);
    }

    pub fn clear_mvp_light_valid(&mut self) {
        self.mvp_valid = false;
        self.lights_valid = false;
    }

    pub fn calculate_mvp_matrix(&mut self) {
        if !self.mvp_valid {
            self.modelview_projection_matrix =
                self.projection_matrix * self.matrix_stack[self.matrix_stack_pointer - 1];
            self.mvp_valid = true;
        }
    }

    pub fn set_num_lights(&mut self, num_lights: u8) {
        self.num_lights = num_lights;
        self.lights_valid = false;
    }
}
