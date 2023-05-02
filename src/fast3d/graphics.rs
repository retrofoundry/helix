use std::any::Any;

use wgpu::BlendState;

#[repr(C)]
pub struct ShaderProgram {
    pub shader_id: u32,
    pub num_inputs: u8,
    pub used_textures: [bool; 2],
    // .. ommiting the rest
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None = 0x00000000,
    Front = 0x00000001,
    Back = 0x00000002,
    FrontAndBack = 0x00000003,
}

pub trait GraphicsAPI {
    fn as_any(&self) -> &dyn Any;

    fn z_is_from_0_to_1(&self) -> bool;
    fn unload_shader(&self, shader: *mut ShaderProgram);
    fn load_shader(&self, shader: *mut ShaderProgram);
    fn create_and_load_new_shader(&self, id: u32) -> *mut ShaderProgram;
    fn lookup_shader(&self, id: u32) -> *mut ShaderProgram;
    fn shader_get_info(&self, shader: *mut ShaderProgram, info: *mut u8, info_size: [bool; 2]);
    fn new_texture(&self) -> u32;
    fn select_texture(&self, unit: i32, id: u32);
    fn upload_texture(&self, data: *const u8, width: i32, height: i32);
    fn set_sampler_parameters(&self, unit: i32, linear: bool, s: u32, t: u32);
    fn set_depth_test(&self, enable: bool);
    fn set_depth_compare(&self, compare: u8);
    fn set_depth_write(&self, enable: bool);
    fn set_polygon_offset(&self, enable: bool);
    fn set_viewport(&self, x: i32, y: i32, width: i32, height: i32);
    fn set_scissor(&self, x: i32, y: i32, width: i32, height: i32);
    fn set_blend_state(&self, blend_state: BlendState);
    fn set_cull_mode(&self, cull_mode: CullMode);
    fn draw_triangles(&self, vertices: *const f32, count: usize, stride: usize);
    fn init(&self);
    fn on_resize(&self);
    fn start_frame(&self);
    fn end_frame(&self);
    fn finish_render(&self);
}

pub struct GraphicsContext {
    pub api: Box<dyn GraphicsAPI>,
}

impl GraphicsContext {
    pub fn new(api: Box<dyn GraphicsAPI>) -> GraphicsContext {
        GraphicsContext { api }
    }
}

// MARK: - DummyGraphicsDevice

pub struct DummyGraphicsDevice {}

impl GraphicsAPI for DummyGraphicsDevice {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn z_is_from_0_to_1(&self) -> bool {
        false
    }
    fn unload_shader(&self, _shader: *mut ShaderProgram) {}
    fn load_shader(&self, _shader: *mut ShaderProgram) {}
    fn create_and_load_new_shader(&self, _id: u32) -> *mut ShaderProgram {
        std::ptr::null_mut()
    }
    fn lookup_shader(&self, _id: u32) -> *mut ShaderProgram {
        std::ptr::null_mut()
    }
    fn shader_get_info(&self, _shader: *mut ShaderProgram, _info: *mut u8, _info_size: [bool; 2]) {}
    fn new_texture(&self) -> u32 {
        0
    }
    fn select_texture(&self, _unit: i32, _id: u32) {}
    fn upload_texture(&self, _data: *const u8, _width: i32, _height: i32) {}
    fn set_sampler_parameters(&self, _unit: i32, _linear: bool, _s: u32, _t: u32) {}
    fn set_depth_test(&self, _enable: bool) {}
    fn set_depth_compare(&self, _compare: u8) {}
    fn set_depth_write(&self, _enable: bool) {}
    fn set_polygon_offset(&self, _enable: bool) {}
    fn set_viewport(&self, _x: i32, _y: i32, _width: i32, _height: i32) {}
    fn set_scissor(&self, _x: i32, _y: i32, _width: i32, _height: i32) {}
    fn set_blend_state(&self, _blend_state: BlendState) {}
    fn set_cull_mode(&self, _cull_mode: CullMode) {}
    fn draw_triangles(&self, _vertices: *const f32, _count: usize, _stride: usize) {}
    fn init(&self) {}
    fn on_resize(&self) {}
    fn start_frame(&self) {}
    fn end_frame(&self) {}
    fn finish_render(&self) {}
}

// MARK: - Native C Bridge

#[repr(C)]
pub struct CGraphicsDevice {
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
    pub set_depth_compare: extern "C" fn(u8),
    pub set_depth_write: extern "C" fn(bool),
    pub set_polygon_offset: extern "C" fn(bool),
    pub set_viewport: extern "C" fn(i32, i32, i32, i32),
    pub set_scissor: extern "C" fn(i32, i32, i32, i32),
    pub set_blend_state: extern "C" fn(BlendState),
    pub set_cull_mode: extern "C" fn(CullMode),
    pub draw_triangles: extern "C" fn(*const f32, usize, usize),
    pub init: extern "C" fn(),
    pub on_resize: extern "C" fn(),
    pub start_frame: extern "C" fn(),
    pub end_frame: extern "C" fn(),
    pub finish_render: extern "C" fn(),
}

pub struct ExternGraphicsDevice {
    pub inner: *mut CGraphicsDevice,
}

impl ExternGraphicsDevice {
    pub fn new(inner: *mut CGraphicsDevice) -> ExternGraphicsDevice {
        ExternGraphicsDevice { inner }
    }
}

impl GraphicsAPI for ExternGraphicsDevice {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn z_is_from_0_to_1(&self) -> bool {
        unsafe { ((*self.inner).z_is_from_0_to_1)() }
    }

    fn unload_shader(&self, shader: *mut ShaderProgram) {
        unsafe { ((*self.inner).unload_shader)(shader) }
    }

    fn load_shader(&self, shader: *mut ShaderProgram) {
        unsafe { ((*self.inner).load_shader)(shader) }
    }

    fn create_and_load_new_shader(&self, id: u32) -> *mut ShaderProgram {
        unsafe { ((*self.inner).create_and_load_new_shader)(id) }
    }

    fn lookup_shader(&self, id: u32) -> *mut ShaderProgram {
        unsafe { ((*self.inner).lookup_shader)(id) }
    }

    fn shader_get_info(&self, shader: *mut ShaderProgram, info: *mut u8, info_size: [bool; 2]) {
        unsafe { ((*self.inner).shader_get_info)(shader, info, info_size) }
    }

    fn new_texture(&self) -> u32 {
        unsafe { ((*self.inner).new_texture)() }
    }

    fn select_texture(&self, unit: i32, id: u32) {
        unsafe { ((*self.inner).select_texture)(unit, id) }
    }

    fn upload_texture(&self, data: *const u8, width: i32, height: i32) {
        unsafe { ((*self.inner).upload_texture)(data, width, height) }
    }

    fn set_sampler_parameters(&self, unit: i32, linear: bool, s: u32, t: u32) {
        unsafe { ((*self.inner).set_sampler_parameters)(unit, linear, s, t) }
    }

    fn set_depth_test(&self, enable: bool) {
        unsafe { ((*self.inner).set_depth_test)(enable) }
    }

    fn set_depth_compare(&self, compare: u8) {
        unsafe { ((*self.inner).set_depth_compare)(compare) }
    }

    fn set_depth_write(&self, enable: bool) {
        unsafe { ((*self.inner).set_depth_write)(enable) }
    }

    fn set_polygon_offset(&self, enable: bool) {
        unsafe { ((*self.inner).set_polygon_offset)(enable) }
    }

    fn set_viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { ((*self.inner).set_viewport)(x, y, width, height) }
    }

    fn set_scissor(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { ((*self.inner).set_scissor)(x, y, width, height) }
    }

    fn set_blend_state(&self, blend_state: BlendState) {
        unsafe { ((*self.inner).set_blend_state)(blend_state) }
    }

    fn set_cull_mode(&self, cull_mode: CullMode) {
        unsafe { ((*self.inner).set_cull_mode)(cull_mode) }
    }

    fn draw_triangles(&self, vertices: *const f32, count: usize, stride: usize) {
        unsafe { ((*self.inner).draw_triangles)(vertices, count, stride) }
    }

    fn init(&self) {
        unsafe { ((*self.inner).init)() }
    }

    fn on_resize(&self) {
        unsafe { ((*self.inner).on_resize)() }
    }

    fn start_frame(&self) {
        unsafe { ((*self.inner).start_frame)() }
    }

    fn end_frame(&self) {
        unsafe { ((*self.inner).end_frame)() }
    }

    fn finish_render(&self) {
        unsafe { ((*self.inner).finish_render)() }
    }
}

#[no_mangle]
pub extern "C" fn GfxCreateContext(device: *mut CGraphicsDevice) -> Box<GraphicsContext> {
    let device = ExternGraphicsDevice::new(device);
    Box::new(GraphicsContext::new(Box::new(device)))
}

#[no_mangle]
pub extern "C" fn GfxGetDevice(gfx_context: Option<&mut GraphicsContext>) -> *mut CGraphicsDevice {
    let context = gfx_context.unwrap();
    context
        .api
        .as_any()
        .downcast_ref::<ExternGraphicsDevice>()
        .unwrap()
        .inner
}
