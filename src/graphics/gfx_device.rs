
#[repr(C)]
pub struct ShaderProgram;

#[repr(C)]
pub struct C_GfxDevice {
    pub z_is_from_0_to_1: extern "C" fn() -> bool,
    pub unload_shader: extern "C" fn(*mut ShaderProgram),
    pub load_shader: extern "C" fn(*mut ShaderProgram),
    pub create_and_load_new_shader: extern "C" fn(u32) -> *mut ShaderProgram,
    pub lookup_shader: extern "C" fn(u32) -> *mut ShaderProgram,
    pub shader_get_info: extern "C" fn(*mut ShaderProgram, *mut u8, [bool; 2]),
    pub new_texture: extern "C" fn() -> u32,
    pub select_texture: extern "C" fn(i32, u32),
    pub upload_texture: extern "C" fn(*const u8, i32, i32),
    pub set_sampler_parameters: extern "C" fn(i32, bool, u32, u32),
    pub set_depth_test: extern "C" fn(bool),
    pub set_depth_mask: extern "C" fn(bool),
    pub set_zmode_decal: extern "C" fn(bool),
    pub set_viewport: extern "C" fn(i32, i32, i32, i32),
    pub set_scissor: extern "C" fn(i32, i32, i32, i32),
    pub set_use_alpha: extern "C" fn(bool),
    pub draw_triangles: extern "C" fn(*const f32, usize, usize),
    pub init: extern "C" fn(),
    pub on_resize: extern "C" fn(),
    pub start_frame: extern "C" fn(),
    pub end_frame: extern "C" fn(),
    pub finish_render: extern "C" fn(),
}

pub struct GfxDevice {
    pub storage: *mut C_GfxDevice,
}

impl GfxDevice {
    pub fn new(storage: *mut C_GfxDevice) -> Self {
        Self { storage }
    }

    pub fn z_is_from_0_to_1(&self) -> bool {
        unsafe { ((*self.storage).z_is_from_0_to_1)() }
    }

    pub fn unload_shader(&self, shader: *mut ShaderProgram) {
        unsafe { ((*self.storage).unload_shader)(shader) }
    }

    pub fn load_shader(&self, shader: *mut ShaderProgram) {
        unsafe { ((*self.storage).load_shader)(shader) }
    }

    pub fn create_and_load_new_shader(&self, id: u32) -> *mut ShaderProgram {
        unsafe { ((*self.storage).create_and_load_new_shader)(id) }
    }

    pub fn lookup_shader(&self, id: u32) -> *mut ShaderProgram {
        unsafe { ((*self.storage).lookup_shader)(id) }
    }

    pub fn shader_get_info(
        &self,
        shader: *mut ShaderProgram,
        info: *mut u8,
        info_size: [bool; 2],
    ) {
        unsafe { ((*self.storage).shader_get_info)(shader, info, info_size) }
    }

    pub fn new_texture(&self) -> u32 {
        unsafe { ((*self.storage).new_texture)() }
    }

    pub fn select_texture(&self, unit: i32, id: u32) {
        unsafe { ((*self.storage).select_texture)(unit, id) }
    }

    pub fn upload_texture(&self, data: *const u8, width: i32, height: i32) {
        unsafe { ((*self.storage).upload_texture)(data, width, height) }
    }

    pub fn set_sampler_parameters(&self, unit: i32, linear: bool, s: u32, t: u32) {
        unsafe { ((*self.storage).set_sampler_parameters)(unit, linear, s, t) }
    }

    pub fn set_depth_test(&self, enable: bool) {
        unsafe { ((*self.storage).set_depth_test)(enable) }
    }

    pub fn set_depth_mask(&self, enable: bool) {
        unsafe { ((*self.storage).set_depth_mask)(enable) }
    }

    pub fn set_zmode_decal(&self, enable: bool) {
        unsafe { ((*self.storage).set_zmode_decal)(enable) }
    }

    pub fn set_viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { ((*self.storage).set_viewport)(x, y, width, height) }
    }

    pub fn set_scissor(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { ((*self.storage).set_scissor)(x, y, width, height) }
    }

    pub fn set_use_alpha(&self, enable: bool) {
        unsafe { ((*self.storage).set_use_alpha)(enable) }
    }

    pub fn draw_triangles(&self, vertices: *const f32, count: usize, stride: usize) {
        unsafe { ((*self.storage).draw_triangles)(vertices, count, stride) }
    }

    pub fn init(&self) {
        unsafe { ((*self.storage).init)() }
    }

    pub fn on_resize(&self) {
        unsafe { ((*self.storage).on_resize)() }
    }

    pub fn start_frame(&self) {
        unsafe { ((*self.storage).start_frame)() }
    }

    pub fn end_frame(&self) {
        unsafe { ((*self.storage).end_frame)() }
    }

    pub fn finish_render(&self) {
        unsafe { ((*self.storage).finish_render)() }
    }
}
