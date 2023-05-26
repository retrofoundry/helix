use std::{any::Any, num::NonZeroU32};

use imgui_glow_renderer::glow::{self, HasContext, NativeProgram, NativeTexture, Program};
use wgpu::CompareFunction;

use crate::fast3d::gbi::defines::G_TX;

use super::{GraphicsAPI, ShaderProgram};

pub struct OpenGLGraphicsDevice {
    pub frame_count: i32,
    pub current_height: i32,
}

impl OpenGLGraphicsDevice {
    fn new() -> Self {
        Self {
            frame_count: 0,
            current_height: 0,
        }
    }

    fn gfx_cm_to_opengl(val: u32) -> i32 {
        if val & G_TX::CLAMP as u32 != 0 {
            return glow::CLAMP_TO_EDGE as i32;
        }

        if val & G_TX::MIRROR as u32 != 0 {
            return glow::MIRRORED_REPEAT as i32;
        }

        glow::REPEAT as i32
    }
}

impl GraphicsAPI for OpenGLGraphicsDevice {
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn z_is_from_0_to_1(&self) -> bool {
        false
    }

    fn unload_shader(&self, gl: &glow::Context, shader: *mut ShaderProgram) {
        if !shader.is_null() {
            unsafe {
                for i in 0..(*shader).num_attribs {
                    gl.disable_vertex_attrib_array((*shader).attrib_locations[i as usize] as u32);
                }
            }
        }
    }

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
    ) -> *mut ShaderProgram {
        unsafe {
            let mut shaders = [
                (glow::VERTEX_SHADER, vertex, None),
                (glow::FRAGMENT_SHADER, fragment, None),
            ];

            let program = gl.create_program().expect("Cannot create program");

            for (kind, source, handle) in &mut shaders {
                let shader = gl.create_shader(*kind).expect("Cannot create shader");
                gl.shader_source(shader, source);
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

            // Grab the locations of the attributes
            let mut count: usize = 0;

            let mut shader_program = ShaderProgram::new();
            shader_program.attrib_locations[count] =
                gl.get_attrib_location(program, "aVtxPos").unwrap() as i32;
            shader_program.attrib_sizes[count] = 4;
            count += 1;

            if uses_tex0 || uses_tex1 {
                shader_program.attrib_locations[count] =
                    gl.get_attrib_location(program, "aTexCoord").unwrap() as i32;
                shader_program.attrib_sizes[count] = 2;
                count += 1;
            }

            if uses_fog {
                shader_program.attrib_locations[count] =
                    gl.get_attrib_location(program, "aFog").unwrap() as i32;
                shader_program.attrib_sizes[count] = 1;
                count += 1;
            }

            for i in 0..num_inputs {
                shader_program.attrib_locations[count] = gl
                    .get_attrib_location(program, &format!("aInput{}", i))
                    .unwrap() as i32;
                shader_program.attrib_sizes[count] = if uses_alpha { 4 } else { 3 };
                count += 1;
            }

            shader_program.shader_id = program.0.into();
            shader_program.num_attribs = count as u8;
            shader_program.num_floats = num_floats as u8;

            // Handle uniforms

            if uses_tex0 {
                let sampler_location = gl.get_uniform_location(program, "uTex0").unwrap();
                gl.uniform_1_i32(Some(&sampler_location), 0);
            }

            if uses_tex1 {
                let sampler_location = gl.get_uniform_location(program, "uTex1").unwrap();
                gl.uniform_1_i32(Some(&sampler_location), 1);
            }

            if uses_alpha && uses_noise {
                shader_program.used_noise = true;
                shader_program.noise_location =
                    gl.get_uniform_location(program, "uNoise").unwrap().0 as i32;
                shader_program.noise_scale_location =
                    gl.get_uniform_location(program, "uNoiseScale").unwrap().0 as i32;
            } else {
                shader_program.used_noise = false;
            }

            Box::into_raw(Box::new(shader_program))
        }
    }

    fn load_shader(&self, gl: &glow::Context, shader: *mut ShaderProgram) {
        unsafe {
            let program = NativeProgram(NonZeroU32::new((*shader).opengl_program_id).unwrap());
            gl.use_program(Some(program));

            // enable vertex attribs and set pointers
            let mut position = 0;
            for i in 0..(*shader).num_attribs {
                gl.enable_vertex_attrib_array((*shader).attrib_locations[i as usize] as u32);
                gl.vertex_attrib_pointer_f32(
                    (*shader).attrib_locations[i as usize] as u32,
                    (*shader).attrib_sizes[i as usize] as i32,
                    glow::FLOAT,
                    false,
                    (*shader).num_floats as i32 * 4,
                    position * 4,
                );

                position += (*shader).attrib_sizes[i as usize] as i32;
            }

            // set uniforms
            if (*shader).used_noise {
                // TODO: verify this works and if so we don't need to store uniform locations into shader program object
                gl.uniform_1_i32(
                    Some(&gl.get_uniform_location(program, "uNoise").unwrap()),
                    self.frame_count,
                );
                gl.uniform_1_i32(
                    Some(&gl.get_uniform_location(program, "uNoiseScale").unwrap()),
                    self.current_height,
                );
            }
        };
    }

    fn new_texture(&self, gl: &glow::Context) -> u32 {
        unsafe { gl.create_texture().unwrap().0.into() }
    }

    fn select_texture(&self, gl: &glow::Context, tile: i32, id: u32) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + tile as u32);
            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(NativeTexture(NonZeroU32::new(id).unwrap())),
            );
        }
    }

    fn upload_texture(&self, gl: &glow::Context, data: *const u8, width: i32, height: i32) {
        unsafe {
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(std::slice::from_raw_parts(
                    data,
                    (width * height * 4) as usize,
                )),
            );
        }
    }

    fn set_sampler_parameters(&self, gl: &glow::Context, tile: i32, linear: bool, s: u32, t: u32) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + tile as u32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                if linear {
                    glow::LINEAR as i32
                } else {
                    glow::NEAREST as i32
                },
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                if linear {
                    glow::LINEAR as i32
                } else {
                    glow::NEAREST as i32
                },
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                Self::gfx_cm_to_opengl(s),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                Self::gfx_cm_to_opengl(t),
            );
        }
    }

    fn set_depth_test(&self, gl: &glow::Context, enable: bool) {
        unsafe {
            if enable {
                gl.enable(glow::DEPTH_TEST);
            } else {
                gl.disable(glow::DEPTH_TEST);
            }
        }
    }

    fn set_depth_compare(&self, gl: &glow::Context, compare: CompareFunction) {
        unsafe {
            match compare {
                CompareFunction::Never => gl.depth_func(glow::NEVER),
                CompareFunction::Less => gl.depth_func(glow::LESS),
                CompareFunction::Equal => gl.depth_func(glow::EQUAL),
                CompareFunction::LessEqual => gl.depth_func(glow::LEQUAL),
                CompareFunction::Greater => gl.depth_func(glow::GREATER),
                CompareFunction::NotEqual => gl.depth_func(glow::NOTEQUAL),
                CompareFunction::GreaterEqual => gl.depth_func(glow::GEQUAL),
                CompareFunction::Always => gl.depth_func(glow::ALWAYS),
            }
        }
    }

    fn set_depth_write(&self, gl: &glow::Context, enable: bool) {
        unsafe {
            gl.depth_mask(enable);
        }
    }

    fn set_polygon_offset(&self, gl: &glow::Context, enable: bool) {
        unsafe {
            if enable {
                gl.polygon_offset(-2.0, 2.0);
                gl.enable(glow::POLYGON_OFFSET_FILL);
            } else {
                gl.polygon_offset(0.0, 0.0);
                gl.disable(glow::POLYGON_OFFSET_FILL);
            }
        }
    }

    fn set_viewport(&mut self, gl: &glow::Context, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            gl.viewport(x, y, width, height);
        }

        self.current_height = height;
    }

    fn set_scissor(&self, x: i32, y: i32, width: i32, height: i32) {
        todo!()
    }

    fn set_blend_state(&self, enabled: bool, blend_state: wgpu::BlendState) {
        todo!()
    }

    fn set_cull_mode(&self, cull_mode: super::CullMode) {
        todo!()
    }

    fn draw_triangles(&self, vertices: *const f32, count: usize, stride: usize) {
        todo!()
    }

    fn init(&self) {
        todo!()
    }

    fn on_resize(&self) {
        todo!()
    }

    fn start_frame(&self) {
        todo!()
    }

    fn end_frame(&self) {
        todo!()
    }

    fn finish_render(&self) {
        todo!()
    }
}
