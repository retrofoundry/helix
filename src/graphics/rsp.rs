use glam::Mat4;

use super::gbi::defines::Light;

pub const MATRIX_STACK_SIZE: usize = 11;

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
