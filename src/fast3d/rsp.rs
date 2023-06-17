use crate::fast3d::gbi::defines::DirLight;

use super::gbi::defines::Light;
use glam::{Mat4, Vec2, Vec3A, Vec4};

pub const MATRIX_STACK_SIZE: usize = 32;
pub const MAX_VERTICES: usize = 256;
pub const MAX_LIGHTS: usize = 7;
pub const MAX_SEGMENTS: usize = 16;

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
    pub uv: Vec2,
    pub color: Vec4,
    pub clip_reject: u8,
}

impl StagingVertex {
    pub const ZERO: Self = Self {
        position: Position::ZERO,
        uv: Vec2::ZERO,
        color: Vec4::ZERO,
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
    pub modelview_projection_matrix_changed: bool,

    pub lights_valid: bool,
    pub num_lights: u8,
    pub lights: [Light; MAX_LIGHTS + 1],
    pub lookat: [Vec3A; 2], // lookat_x, lookat_y

    pub fog_multiplier: i16,
    pub fog_offset: i16,

    pub vertex_table: [StagingVertex; MAX_VERTICES + 4],

    pub lights_coeffs: [Vec3A; MAX_LIGHTS],
    pub lookat_coeffs: [Vec3A; 2], // lookat_x, lookat_y

    pub segments: [usize; MAX_SEGMENTS],
}

impl RSP {
    pub fn new() -> Self {
        RSP {
            geometry_mode: 0,
            projection_matrix: Mat4::ZERO,

            matrix_stack: [Mat4::ZERO; MATRIX_STACK_SIZE],
            matrix_stack_pointer: 0,

            modelview_projection_matrix: Mat4::ZERO,
            modelview_projection_matrix_changed: false,

            lights_valid: true,
            num_lights: 0,
            lights: [Light::ZERO; MAX_LIGHTS + 1],
            lookat: [Vec3A::ZERO; 2],

            fog_multiplier: 0,
            fog_offset: 0,

            vertex_table: [StagingVertex::ZERO; MAX_VERTICES + 4],

            lights_coeffs: [Vec3A::ZERO; MAX_LIGHTS],
            lookat_coeffs: [Vec3A::ZERO; 2],

            segments: [0; MAX_SEGMENTS],
        }
    }

    pub fn reset(&mut self) {
        self.matrix_stack_pointer = 1;
        self.set_num_lights(1);
    }

    pub fn recompute_mvp_matrix(&mut self) {
        self.modelview_projection_matrix =
            self.matrix_stack[self.matrix_stack_pointer - 1] * self.projection_matrix;
    }

    pub fn set_num_lights(&mut self, num_lights: u8) {
        self.num_lights = num_lights;
        self.lights_valid = false;
    }

    pub fn set_segment(&mut self, segment: usize, address: usize) {
        assert!(segment < MAX_SEGMENTS);
        self.segments[segment] = address;
    }

    pub fn set_fog(&mut self, multiplier: i16, offset: i16) {
        self.fog_multiplier = multiplier;
        self.fog_offset = offset;
    }

    pub fn set_light_color(&mut self, index: usize, value: u32) {
        assert!(index <= MAX_LIGHTS);

        let light = &mut self.lights[index];
        unsafe {
            light.raw.words[0] = value;
        }
        unsafe {
            light.raw.words[1] = value;
        }
        self.lights_valid = false;
    }

    pub fn from_segmented(&self, address: usize) -> usize {
        let segment = (address >> 24) & 0x0F;
        let offset = address & 0x00FFFFFF;

        if self.segments[segment] != 0 {
            self.segments[segment] + offset
        } else {
            address
        }
    }

    pub fn set_clip_ratio(&mut self, _ratio: usize) {
        // TODO: implement
    }

    pub fn set_persp_norm(&mut self, _norm: usize) {
        // TODO: implement
    }

    pub fn set_light(&mut self, index: usize, address: usize) {
        assert!(index <= MAX_LIGHTS);

        let data = self.from_segmented(address);
        let light_ptr = data as *const Light;
        let light = unsafe { &*light_ptr };

        self.lights[index] = *light;
        self.lights_valid = false;
    }

    pub fn set_look_at(&mut self, index: usize, address: usize) {
        assert!(index < 2);
        let data = self.from_segmented(address);
        let dir_light_ptr = data as *const DirLight;
        let dir_light = unsafe { &*dir_light_ptr };

        let lookat = if index == 0 {
            &mut self.lookat[0]
        } else {
            &mut self.lookat[1]
        };
        if dir_light.dir[0] != 0 || dir_light.dir[1] != 0 || dir_light.dir[2] != 0 {
            *lookat = Vec3A::new(
                dir_light.dir[0] as f32,
                dir_light.dir[1] as f32,
                dir_light.dir[2] as f32,
            )
            .normalize();
        } else {
            *lookat = Vec3A::ZERO;
        }
    }
}
