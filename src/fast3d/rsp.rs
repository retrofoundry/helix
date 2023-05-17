use super::{gbi::defines::Light, utils::color::Color};
use glam::{Mat4, Vec3A};

pub const MATRIX_STACK_SIZE: usize = 11;
pub const MAX_VERTICES: usize = 64;

// excluding ambient light
pub const MAX_LIGHTS: usize = 7;

#[repr(C)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Position {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
}

#[repr(C)]
pub struct StagingVertex {
    pub position: Position,
    pub uv: [f32; 2],
    pub color: Color,
    pub clip_reject: u8,
}

impl StagingVertex {
    pub const ZERO: Self = Self {
        position: Position::ZERO,
        uv: [0.0; 2],
        color: Color::TRANSPARENT,
        clip_reject: 0,
    };
}

#[cfg(feature = "f3dex2")]
pub enum RSPGeometry {
    G_ZBUFFER = 1 << 0,
    G_SHADE = 1 << 2,
    G_TEXTURE_ENABLE = 0,
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
    pub projection_matrix: Mat4,

    pub matrix_stack: [Mat4; MATRIX_STACK_SIZE],
    pub matrix_stack_pointer: usize,

    pub modelview_projection_matrix: Mat4,

    pub lights_valid: bool,
    pub num_lights: u8,
    pub lights: [Light; MAX_LIGHTS + 1],

    pub fog_multiplier: i16,
    pub fog_offset: i16,

    pub vertex_table: [StagingVertex; MAX_VERTICES + 4],

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

            modelview_projection_matrix: Mat4::ZERO,

            lights_valid: true,
            num_lights: 0,
            lights: [Light::ZERO; MAX_LIGHTS + 1],

            fog_multiplier: 0,
            fog_offset: 0,

            vertex_table: [StagingVertex::ZERO; MAX_VERTICES + 4],

            lights_coeffs: [Vec3A::ZERO; MAX_LIGHTS],
            lookat_coeffs: [Vec3A::ZERO; 2],
        }
    }

    pub fn reset(&mut self) {
        self.matrix_stack_pointer = 1;
        self.set_num_lights(2);
    }

    pub fn recompute_mvp_matrix(&mut self) {
        self.modelview_projection_matrix =
            self.matrix_stack[self.matrix_stack_pointer - 1] * self.projection_matrix;
    }

    pub fn set_num_lights(&mut self, num_lights: u8) {
        self.num_lights = num_lights;
        self.lights_valid = false;
    }
}
