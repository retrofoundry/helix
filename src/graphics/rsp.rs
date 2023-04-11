use glam::Mat4;

use super::gbi::defines::Light;

pub const MATRIX_STACK_SIZE: usize = 11;

// excluding ambient light
pub const MAX_LIGHTS: usize = 7;

pub struct RSP {
    pub geometry_mode: u32,
    pub projection_matrix: Mat4,

    pub matrix_stack: [Mat4; MATRIX_STACK_SIZE], // Make it fixed size
    pub matrix_stack_pointer: usize,

    pub mvp_valid: bool,
    pub modelview_projection_matrix: Mat4,

    pub lights_valid: bool,
    pub lights: [Light; MAX_LIGHTS + 1],
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
            lights: [Light::ZERO; MAX_LIGHTS + 1],
        }
    }

    pub fn reset(&mut self) {}

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
}
