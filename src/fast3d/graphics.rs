use imgui_glow_renderer::glow;
use std::any::Any;
use wgpu::{BlendState, CompareFunction};

pub mod opengl_device;
pub mod opengl_program;

#[repr(C)]
#[derive(Debug)]
pub struct CompiledProgram {
    pub shader_id: u32,
    pub opengl_program_id: u32,
    pub num_floats: u8,
    pub attrib_locations: [i32; 7],
    pub attrib_sizes: [u8; 7],
    pub num_attribs: u8,
    pub used_noise: bool,
    pub noise_location: i32,
    pub noise_scale_location: i32,
}

impl CompiledProgram {
    pub fn new() -> Self {
        CompiledProgram {
            shader_id: 0,
            opengl_program_id: 0,
            num_floats: 0,
            attrib_locations: [0; 7],
            attrib_sizes: [0; 7],
            num_attribs: 0,
            used_noise: false,
            noise_location: 0,
            noise_scale_location: 0,
        }
    }
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
    fn unload_shader(&self, gl: &glow::Context, shader: &CompiledProgram);
    fn new_shader(
        &self,
        gl: &glow::Context,
        vertex: String,
        fragment: String,
        num_floats: usize,
        uses_tex0: bool,
        uses_tex1: bool,
        uses_fog: bool,
        uses_alpha: bool,
        uses_noise: bool,
        num_inputs: u8,
    ) -> CompiledProgram;
    fn load_shader(&self, gl: &glow::Context, shader: &CompiledProgram);
    fn new_texture(&self, gl: &glow::Context) -> u32;
    fn select_texture(&self, gl: &glow::Context, unit: i32, id: u32);
    fn upload_texture(&self, gl: &glow::Context, data: *const u8, width: i32, height: i32);
    fn set_sampler_parameters(&self, gl: &glow::Context, unit: i32, linear: bool, s: u32, t: u32);
    fn set_depth_test(&self, gl: &glow::Context, enable: bool);
    fn set_depth_compare(&self, gl: &glow::Context, compare: CompareFunction);
    fn set_depth_write(&self, gl: &glow::Context, enable: bool);
    fn set_polygon_offset(&self, _gl: &glow::Context, enable: bool);
    fn set_viewport(&mut self, _gl: &glow::Context, x: i32, y: i32, width: i32, height: i32);
    fn set_scissor(&self, gl: &glow::Context, x: i32, y: i32, width: i32, height: i32);
    fn set_blend_state(&self, gl: &glow::Context, enabled: bool, blend_state: BlendState);
    fn set_cull_mode(&self, gl: &glow::Context, cull_mode: Option<wgpu::Face>);
    fn draw_triangles(&self, gl: &glow::Context, vertices: *const f32, count: usize, stride: usize);
    fn init(&self);
    fn on_resize(&self);
    fn start_frame(&mut self, gl: &glow::Context);
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
