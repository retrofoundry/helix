use glam::Mat4;

pub const MATRIX_STACK_SIZE: usize = 11;

pub struct RSP {
    pub geometry_mode: u32,
    pub projection_matrix: Mat4,

    pub matrix_stack: [Mat4; MATRIX_STACK_SIZE], // Make it fixed size
    pub matrix_stack_index: usize,

    pub mvp_valid: bool,
    pub modelview_projection_matrix: Mat4,

    pub lights_valid: bool,
}

impl RSP {
    pub fn new() -> Self {
        RSP {
            geometry_mode: 0,
            projection_matrix: Mat4::ZERO,

            matrix_stack: [Mat4::ZERO; MATRIX_STACK_SIZE],
            matrix_stack_index: 0,

            mvp_valid: true,
            modelview_projection_matrix: Mat4::ZERO,

            lights_valid: true,
        }
    }

    pub fn reset(&mut self) {}

    pub fn clear_mvp_light_valid(&mut self) {
        self.mvp_valid = false;
        self.lights_valid = false;
    }
}
