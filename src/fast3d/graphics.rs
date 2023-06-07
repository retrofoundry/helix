use imgui_glow_renderer::glow;
use std::any::Any;
use wgpu::{BlendState, CompareFunction};

use self::opengl_program::OpenGLProgram;

use super::utils::color::Color;

pub mod opengl_device;
pub mod opengl_program;

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
    fn unload_program(&self, gl: &glow::Context, program: &OpenGLProgram);
    fn compile_program(&self, gl: &glow::Context, program: &mut OpenGLProgram);
    fn load_program(&mut self, gl: &glow::Context, program: &OpenGLProgram);
    fn new_texture(&self, gl: &glow::Context) -> glow::NativeTexture;
    fn select_texture(&self, gl: &glow::Context, unit: i32, texture: glow::NativeTexture);
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
    fn set_uniforms(&self, gl: &glow::Context, fog_color: &Color, blend_color: &Color);
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
