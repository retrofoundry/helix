use std::any::Any;
use wgpu::BlendState;

pub mod dummy_device;
mod extern_device;
pub mod opengl_program;

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
    fn as_any_ref(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn z_is_from_0_to_1(&self) -> bool;
    fn unload_shader(&self, shader: *mut ShaderProgram);
    fn load_shader(&self, shader: *mut ShaderProgram);
    fn create_and_load_new_shader(&self, id: u32) -> *mut ShaderProgram;
    fn lookup_shader(&self, id: u32) -> *mut ShaderProgram;
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
    fn set_blend_state(&self, enabled: bool, blend_state: BlendState);
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
