use crate::fast3d::graphics::{CullMode, GraphicsAPI, GraphicsContext, ShaderProgram};
use std::any::Any;
use wgpu::BlendState;

#[repr(C)]
pub struct CGraphicsDevice {
    pub z_is_from_0_to_1: extern "C" fn() -> bool,
    pub unload_shader: extern "C" fn(*mut ShaderProgram),
    pub load_shader: extern "C" fn(*mut ShaderProgram),
    pub create_and_load_new_shader: extern "C" fn(u32) -> *mut ShaderProgram,
    pub lookup_shader: extern "C" fn(u32) -> *mut ShaderProgram,
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
    pub set_blend_state: extern "C" fn(bool, BlendState),
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
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
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

    fn set_blend_state(&self, enabled: bool, blend_state: BlendState) {
        unsafe { ((*self.inner).set_blend_state)(enabled, blend_state) }
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

// MARK: - C API

#[no_mangle]
pub extern "C" fn GfxCreateExternContext(device: *mut CGraphicsDevice) -> Box<GraphicsContext> {
    let device = ExternGraphicsDevice::new(device);
    Box::new(GraphicsContext::new(Box::new(device)))
}

#[no_mangle]
pub extern "C" fn GfxGetExternDevice(
    gfx_context: Option<&mut GraphicsContext>,
) -> *mut CGraphicsDevice {
    let context = gfx_context.unwrap();
    context
        .api
        .as_any_ref()
        .downcast_ref::<ExternGraphicsDevice>()
        .unwrap()
        .inner
}
