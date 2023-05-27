use crate::fast3d::graphics::{CullMode, GraphicsAPI, ShaderProgram};
use imgui_glow_renderer::glow::{self, HasContext};
use std::any::Any;
use wgpu::{BlendState, CompareFunction};

pub struct DummyGraphicsDevice {
    pub program: <glow::Context as HasContext>::Program,
    pub vertex_array: <glow::Context as HasContext>::VertexArray,
}

impl DummyGraphicsDevice {
    pub fn new(gl: &glow::Context, shader_header: &str) -> Self {
        const VERTEX_SHADER_SOURCE: &str = r#"
const vec2 verts[3] = vec2[3](
    vec2(0.5f, 1.0f),
    vec2(0.0f, 0.0f),
    vec2(1.0f, 0.0f)
);

out vec2 vert;
out vec4 color;

vec4 srgb_to_linear(vec4 srgb_color) {
    // Calcuation as documented by OpenGL
    vec3 srgb = srgb_color.rgb;
    vec3 selector = ceil(srgb - 0.04045);
    vec3 less_than_branch = srgb / 12.92;
    vec3 greater_than_branch = pow((srgb + 0.055) / 1.055, vec3(2.4));
    return vec4(
        mix(less_than_branch, greater_than_branch, selector),
        srgb_color.a
    );
}

void main() {
    vert = verts[gl_VertexID];
    color = srgb_to_linear(vec4(vert, 0.5, 1.0));
    gl_Position = vec4(vert - 0.5, 0.0, 1.0);
}
"#;
        const FRAGMENT_SHADER_SOURCE: &str = r#"
in vec2 vert;
in vec4 color;

out vec4 frag_color;

vec4 linear_to_srgb(vec4 linear_color) {
    vec3 linear = linear_color.rgb;
    vec3 selector = ceil(linear - 0.0031308);
    vec3 less_than_branch = linear * 12.92;
    vec3 greater_than_branch = pow(linear, vec3(1.0/2.4)) * 1.055 - 0.055;
    return vec4(
        mix(less_than_branch, greater_than_branch, selector),
        linear_color.a
    );
}

void main() {
    frag_color = linear_to_srgb(color);
}
"#;

        let mut shaders = [
            (glow::VERTEX_SHADER, VERTEX_SHADER_SOURCE, None),
            (glow::FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE, None),
        ];

        unsafe {
            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            let program = gl.create_program().expect("Cannot create program");

            for (kind, source, handle) in &mut shaders {
                let shader = gl.create_shader(*kind).expect("Cannot create shader");
                gl.shader_source(shader, &format!("{}\n{}", shader_header, *source));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                *handle = Some(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for &(_, _, shader) in &shaders {
                gl.detach_shader(program, shader.unwrap());
                gl.delete_shader(shader.unwrap());
            }

            Self {
                program,
                vertex_array,
            }
        }
    }

    pub fn render(&self, gl: &glow::Context) {
        unsafe {
            gl.clear_color(0.05, 0.05, 0.1, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }
}

impl GraphicsAPI for DummyGraphicsDevice {
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn z_is_from_0_to_1(&self) -> bool {
        false
    }
    fn unload_shader(&self, _gl: &glow::Context, _shader: *mut ShaderProgram) {}
    fn new_shader(
        &self,
        _gl: &glow::Context,
        vertex: String,
        fragment: String,
        num_floats: usize,
        uses_tex0: bool,
        uses_tex1: bool,
        uses_fog: bool,
        uses_alpha: bool,
        uses_noise: bool,
        num_inputs: u8,
    ) -> *mut ShaderProgram {
        std::ptr::null_mut()
    }
    fn load_shader(&self, _gl: &glow::Context, _shader: *mut ShaderProgram) {}
    fn new_texture(&self, _gl: &glow::Context) -> u32 {
        0
    }
    fn select_texture(&self, _gl: &glow::Context, _unit: i32, _id: u32) {}
    fn upload_texture(&self, _gl: &glow::Context, _data: *const u8, _width: i32, _height: i32) {}
    fn set_sampler_parameters(
        &self,
        _gl: &glow::Context,
        _unit: i32,
        _linear: bool,
        _s: u32,
        _t: u32,
    ) {
    }
    fn set_depth_test(&self, _gl: &glow::Context, _enable: bool) {}
    fn set_depth_compare(&self, _gl: &glow::Context, _compare: CompareFunction) {}
    fn set_depth_write(&self, _gl: &glow::Context, _enable: bool) {}
    fn set_polygon_offset(&self, _gl: &glow::Context, _enable: bool) {}
    fn set_viewport(&mut self, _gl: &glow::Context, _x: i32, _y: i32, _width: i32, _height: i32) {}
    fn set_scissor(&self, _gl: &glow::Context, _x: i32, _y: i32, _width: i32, _height: i32) {}
    fn set_blend_state(&self, _gl: &glow::Context, _enabld: bool, _blend_state: BlendState) {}
    fn set_cull_mode(&self, _gl: &glow::Context, _cull_mode: Option<wgpu::Face>) {}
    fn draw_triangles(
        &self,
        _gl: &glow::Context,
        _vertices: *const f32,
        _count: usize,
        _stride: usize,
    ) {
    }
    fn init(&self) {}
    fn on_resize(&self) {}
    fn start_frame(&mut self, _gl: &glow::Context) {}
    fn end_frame(&self) {}
    fn finish_render(&self) {}
}
