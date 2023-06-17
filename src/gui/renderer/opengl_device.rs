use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    num::NonZeroU32,
};

use glam::{Vec2, Vec3, Vec4};
use imgui_glow_renderer::glow::{self, HasContext};

use crate::fast3d::{
    gbi::defines::G_TX,
    graphics::{
        GraphicsIntermediateSampler, GraphicsIntermediateStencil, GraphicsIntermediateTexture,
    },
    rdp::NUM_TILE_DESCRIPTORS,
    utils::{color_combiner::CombineParams, tile::TileDescriptor},
};

use super::opengl_program::OpenGLProgram;

const FLOAT_SIZE: usize = std::mem::size_of::<f32>();

pub struct OpenGLGraphicsDevice {
    _vbo: <glow::Context as HasContext>::Buffer,
    _vao: <glow::Context as HasContext>::VertexArray,

    shader_cache: HashMap<u64, OpenGLProgram>,
    current_shader: u64,

    frame_count: i32,
    current_height: i32,

    fog_color_location: Option<<glow::Context as HasContext>::UniformLocation>,
    blend_color_location: Option<<glow::Context as HasContext>::UniformLocation>,
    prim_color_location: Option<<glow::Context as HasContext>::UniformLocation>,
    env_color_location: Option<<glow::Context as HasContext>::UniformLocation>,
    key_center_location: Option<<glow::Context as HasContext>::UniformLocation>,
    key_scale_location: Option<<glow::Context as HasContext>::UniformLocation>,
    prim_lod_frac_location: Option<<glow::Context as HasContext>::UniformLocation>,
    k4_location: Option<<glow::Context as HasContext>::UniformLocation>,
    k5_location: Option<<glow::Context as HasContext>::UniformLocation>,
}

impl OpenGLGraphicsDevice {
    pub fn new(gl: &glow::Context) -> Self {
        let _vbo = unsafe { gl.create_buffer().unwrap() };
        let _vao = unsafe { gl.create_vertex_array().unwrap() };

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(_vbo));
            gl.bind_vertex_array(Some(_vao));
        }

        Self {
            _vbo,
            _vao,

            shader_cache: HashMap::new(),
            current_shader: 0,

            frame_count: 0,
            current_height: 0,

            fog_color_location: None,
            blend_color_location: None,
            prim_color_location: None,
            env_color_location: None,
            key_center_location: None,
            key_scale_location: None,
            prim_lod_frac_location: None,
            k4_location: None,
            k5_location: None,
        }
    }

    pub fn gfx_cm_to_opengl(val: u32) -> i32 {
        if val & G_TX::CLAMP as u32 != 0 {
            return glow::CLAMP_TO_EDGE as i32;
        }

        if val & G_TX::MIRROR as u32 != 0 {
            return glow::MIRRORED_REPEAT as i32;
        }

        glow::REPEAT as i32
    }

    pub fn gfx_blend_operation_to_gl(operation: wgpu::BlendOperation) -> u32 {
        match operation {
            wgpu::BlendOperation::Add => glow::FUNC_ADD,
            wgpu::BlendOperation::Subtract => glow::FUNC_SUBTRACT,
            wgpu::BlendOperation::ReverseSubtract => glow::FUNC_REVERSE_SUBTRACT,
            wgpu::BlendOperation::Min => glow::MIN,
            wgpu::BlendOperation::Max => glow::MAX,
        }
    }

    pub fn gfx_blend_factor_to_gl(factor: wgpu::BlendFactor) -> u32 {
        match factor {
            wgpu::BlendFactor::Zero => glow::ZERO,
            wgpu::BlendFactor::One => glow::ONE,
            wgpu::BlendFactor::Src => glow::SRC_COLOR,
            wgpu::BlendFactor::OneMinusSrc => glow::ONE_MINUS_SRC_COLOR,
            wgpu::BlendFactor::SrcAlpha => glow::SRC_ALPHA,
            wgpu::BlendFactor::OneMinusSrcAlpha => glow::ONE_MINUS_SRC_ALPHA,
            wgpu::BlendFactor::Dst => glow::DST_COLOR,
            wgpu::BlendFactor::OneMinusDst => glow::ONE_MINUS_DST_COLOR,
            wgpu::BlendFactor::DstAlpha => glow::DST_ALPHA,
            wgpu::BlendFactor::OneMinusDstAlpha => glow::ONE_MINUS_DST_ALPHA,
            wgpu::BlendFactor::SrcAlphaSaturated => glow::SRC_ALPHA_SATURATE,
            wgpu::BlendFactor::Constant => glow::CONSTANT_COLOR,
            wgpu::BlendFactor::OneMinusConstant => glow::ONE_MINUS_CONSTANT_COLOR,
        }
    }

    fn compile_program(&self, gl: &glow::Context, program: &mut OpenGLProgram) {
        unsafe {
            let mut shaders = [
                (
                    glow::VERTEX_SHADER,
                    program.preprocessed_vertex.clone(),
                    None,
                ),
                (
                    glow::FRAGMENT_SHADER,
                    program.preprocessed_frag.clone(),
                    None,
                ),
            ];

            let native_program = gl.create_program().expect("Cannot create program");

            for (kind, source, handle) in &mut shaders {
                let shader = gl.create_shader(*kind).expect("Cannot create shader");
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }

                gl.attach_shader(native_program, shader);
                *handle = Some(shader);
            }

            gl.link_program(native_program);
            if !gl.get_program_link_status(native_program) {
                panic!("{}", gl.get_program_info_log(native_program));
            }

            // Handle uniforms

            if program.get_define_bool("USE_TEXTURE0") {
                let sampler_location = gl.get_uniform_location(native_program, "uTex0").unwrap();
                gl.uniform_1_i32(Some(&sampler_location), 0);
            }

            if program.get_define_bool("USE_TEXTURE1") {
                let sampler_location = gl.get_uniform_location(native_program, "uTex1").unwrap();
                gl.uniform_1_i32(Some(&sampler_location), 1);
            }

            program.compiled_program = Some(native_program);
        }
    }

    fn unload_program(&self, gl: &glow::Context, program: &OpenGLProgram) {
        unsafe {
            let native_program = program.compiled_program.unwrap();

            let vtx_pos = gl.get_attrib_location(native_program, "aVtxPos").unwrap();
            gl.disable_vertex_attrib_array(vtx_pos);

            let vtx_color = gl.get_attrib_location(native_program, "aVtxColor").unwrap();
            gl.disable_vertex_attrib_array(vtx_color);

            if program.get_define_bool("USE_TEXTURE0") || program.get_define_bool("USE_TEXTURE1") {
                let tex_coord = gl.get_attrib_location(native_program, "aTexCoord").unwrap();
                gl.disable_vertex_attrib_array(tex_coord);
            }
        }
    }

    fn use_program(&mut self, gl: &glow::Context, program: &OpenGLProgram) {
        unsafe {
            let native_program = program.compiled_program;
            gl.use_program(native_program);
            let native_program = native_program.unwrap();

            // Set the vertex attributes
            let mut accumulated_offset = 0;

            let vtx_pos = gl.get_attrib_location(native_program, "aVtxPos").unwrap();
            gl.enable_vertex_attrib_array(vtx_pos);

            let pos_size = 4;
            gl.vertex_attrib_pointer_f32(
                vtx_pos,
                pos_size,
                glow::FLOAT,
                false,
                (program.num_floats * FLOAT_SIZE) as i32,
                0,
            );
            accumulated_offset += pos_size;

            let vtx_color = gl.get_attrib_location(native_program, "aVtxColor").unwrap();
            gl.enable_vertex_attrib_array(vtx_color);

            let color_size = 4;
            gl.vertex_attrib_pointer_f32(
                vtx_color,
                color_size,
                glow::FLOAT,
                false,
                (program.num_floats * FLOAT_SIZE) as i32,
                accumulated_offset * FLOAT_SIZE as i32,
            );
            accumulated_offset += color_size;

            if program.get_define_bool("USE_TEXTURE0") || program.get_define_bool("USE_TEXTURE1") {
                let tex_coord = gl.get_attrib_location(native_program, "aTexCoord").unwrap();
                gl.enable_vertex_attrib_array(tex_coord);

                let coord_size = 2;
                gl.vertex_attrib_pointer_f32(
                    tex_coord,
                    coord_size,
                    glow::FLOAT,
                    false,
                    (program.num_floats * FLOAT_SIZE) as i32,
                    accumulated_offset * FLOAT_SIZE as i32,
                );
            }

            if program.get_define_bool("USE_FOG") {
                self.fog_color_location = gl.get_uniform_location(native_program, "uFogColor");
            }

            self.blend_color_location = gl.get_uniform_location(native_program, "uBlendColor");
            self.prim_color_location = gl.get_uniform_location(native_program, "uPrimColor");
            self.env_color_location = gl.get_uniform_location(native_program, "uEnvColor");
            self.key_center_location = gl.get_uniform_location(native_program, "uKeyCenter");
            self.key_scale_location = gl.get_uniform_location(native_program, "uKeyScale");
            self.prim_lod_frac_location = gl.get_uniform_location(native_program, "uPrimLodFrac");
            self.k4_location = gl.get_uniform_location(native_program, "uK4");
            self.k5_location = gl.get_uniform_location(native_program, "uK5");

            // Set the uniforms
            if program.get_define_bool("USE_ALPHA")
                && program.get_define_bool("ALPHA_COMPARE_DITHER")
            {
                if let Some(frame_count_location) =
                    gl.get_uniform_location(native_program, "uFrameCount")
                {
                    gl.uniform_1_i32(Some(&frame_count_location), self.frame_count);
                }
                if let Some(frame_height_location) =
                    gl.get_uniform_location(native_program, "uFrameHeight")
                {
                    gl.uniform_1_i32(Some(&frame_height_location), self.current_height);
                }
            }
        };
    }

    fn set_depth_compare(&self, gl: &glow::Context, compare: wgpu::CompareFunction) {
        unsafe {
            match compare {
                wgpu::CompareFunction::Never => gl.depth_func(glow::NEVER),
                wgpu::CompareFunction::Less => gl.depth_func(glow::LESS),
                wgpu::CompareFunction::Equal => gl.depth_func(glow::EQUAL),
                wgpu::CompareFunction::LessEqual => gl.depth_func(glow::LEQUAL),
                wgpu::CompareFunction::Greater => gl.depth_func(glow::GREATER),
                wgpu::CompareFunction::NotEqual => gl.depth_func(glow::NOTEQUAL),
                wgpu::CompareFunction::GreaterEqual => gl.depth_func(glow::GEQUAL),
                wgpu::CompareFunction::Always => gl.depth_func(glow::ALWAYS),
            }
        }
    }
}

impl OpenGLGraphicsDevice {
    pub fn load_program(
        &mut self,
        gl: &glow::Context,
        other_mode_h: u32,
        other_mode_l: u32,
        combine: CombineParams,
        tile_descriptors: [TileDescriptor; NUM_TILE_DESCRIPTORS],
    ) {
        // calculate the hash of the shader
        let mut hasher = DefaultHasher::new();

        other_mode_h.hash(&mut hasher);
        other_mode_l.hash(&mut hasher);
        combine.hash(&mut hasher);
        tile_descriptors.hash(&mut hasher);

        let shader_hash = hasher.finish();

        // check if the shader is already loaded
        if self.current_shader == shader_hash {
            return;
        }

        // unload the current shader
        if self.current_shader != 0 {
            let current_shader = self.shader_cache.get(&self.current_shader).unwrap();
            self.current_shader = 0;
            self.unload_program(gl, current_shader);
        }

        // check if the shader is in the cache
        let shader = self.shader_cache.get(&shader_hash).cloned();
        if let Some(shader) = shader {
            self.current_shader = shader_hash;
            self.use_program(gl, &shader);
            return;
        }

        // create the shader and add it to the cache
        let mut program = OpenGLProgram::new(other_mode_h, other_mode_l, combine, tile_descriptors);
        program.init();
        program.preprocess();

        self.compile_program(gl, &mut program);
        self.current_shader = shader_hash;
        self.use_program(gl, &program);
        self.shader_cache.insert(shader_hash, program);
    }

    pub fn bind_texture(
        &self,
        gl: &glow::Context,
        tile: usize,
        texture: &mut GraphicsIntermediateTexture,
    ) {
        // check if we've already uploaded this texture to the GPU
        if let Some(texture_id) = texture.device_id {
            // trace!("Texture found in GPU cache");
            let native_texture = glow::NativeTexture(NonZeroU32::new(texture_id).unwrap());

            unsafe {
                gl.active_texture(glow::TEXTURE0 + tile as u32);
                gl.bind_texture(glow::TEXTURE_2D, Some(native_texture));
            }

            return;
        }

        // Upload the texture to the GPU
        unsafe {
            let native_texture = gl.create_texture().unwrap();

            gl.active_texture(glow::TEXTURE0 + tile as u32);
            gl.bind_texture(glow::TEXTURE_2D, Some(native_texture));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                texture.width as i32,
                texture.height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(std::slice::from_raw_parts(
                    texture.data.as_ptr() as *const u8,
                    (texture.width * texture.height * 4) as usize,
                )),
            );

            // Update cached entry with the GPU texture ID
            texture.device_id = Some(native_texture.0.into());
        }
    }

    pub fn bind_sampler(
        &self,
        gl: &glow::Context,
        tile: usize,
        sampler: &GraphicsIntermediateSampler,
    ) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + tile as u32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                if sampler.linear_filter {
                    glow::LINEAR as i32
                } else {
                    glow::NEAREST as i32
                },
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                if sampler.linear_filter {
                    glow::LINEAR as i32
                } else {
                    glow::NEAREST as i32
                },
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                Self::gfx_cm_to_opengl(sampler.clamp_s),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                Self::gfx_cm_to_opengl(sampler.clamp_t),
            );
        }
    }

    pub fn set_depth_stencil_params(
        &self,
        gl: &glow::Context,
        params: Option<GraphicsIntermediateStencil>,
    ) {
        unsafe {
            if let Some(params) = params {
                gl.enable(glow::DEPTH_TEST);
                gl.depth_mask(params.depth_write_enabled);

                match params.depth_compare {
                    wgpu::CompareFunction::Never => gl.depth_func(glow::NEVER),
                    wgpu::CompareFunction::Less => gl.depth_func(glow::LESS),
                    wgpu::CompareFunction::Equal => gl.depth_func(glow::EQUAL),
                    wgpu::CompareFunction::LessEqual => gl.depth_func(glow::LEQUAL),
                    wgpu::CompareFunction::Greater => gl.depth_func(glow::GREATER),
                    wgpu::CompareFunction::NotEqual => gl.depth_func(glow::NOTEQUAL),
                    wgpu::CompareFunction::GreaterEqual => gl.depth_func(glow::GEQUAL),
                    wgpu::CompareFunction::Always => gl.depth_func(glow::ALWAYS),
                }

                if params.polygon_offset {
                    gl.polygon_offset(-2.0, 2.0);
                    gl.enable(glow::POLYGON_OFFSET_FILL);
                } else {
                    gl.polygon_offset(0.0, 0.0);
                    gl.disable(glow::POLYGON_OFFSET_FILL);
                }
            } else {
                gl.disable(glow::DEPTH_TEST);
            }
        }
    }

    pub fn set_viewport(&mut self, gl: &glow::Context, viewport: &Vec4) {
        unsafe {
            gl.viewport(
                viewport.x as i32,
                viewport.y as i32,
                viewport.z as i32,
                viewport.w as i32,
            );
        }

        self.current_height = viewport.w as i32;
    }

    pub fn set_scissor(&self, gl: &glow::Context, scissor: [u32; 4]) {
        unsafe {
            gl.scissor(
                scissor[0] as i32,
                scissor[1] as i32,
                scissor[2] as i32,
                scissor[3] as i32,
            );
        }
    }

    pub fn set_blend_state(&self, gl: &glow::Context, blend_state: Option<wgpu::BlendState>) {
        unsafe {
            if let Some(blend_state) = blend_state {
                gl.enable(glow::BLEND);

                gl.blend_equation_separate(
                    Self::gfx_blend_operation_to_gl(blend_state.color.operation),
                    Self::gfx_blend_operation_to_gl(blend_state.alpha.operation),
                );

                gl.blend_func_separate(
                    Self::gfx_blend_factor_to_gl(blend_state.color.src_factor),
                    Self::gfx_blend_factor_to_gl(blend_state.color.dst_factor),
                    Self::gfx_blend_factor_to_gl(blend_state.alpha.src_factor),
                    Self::gfx_blend_factor_to_gl(blend_state.alpha.dst_factor),
                );
            } else {
                gl.disable(glow::BLEND);
            }
        }
    }

    pub fn set_cull_mode(&self, gl: &glow::Context, cull_mode: Option<wgpu::Face>) {
        unsafe {
            match cull_mode {
                Some(wgpu::Face::Front) => {
                    gl.enable(glow::CULL_FACE);
                    gl.cull_face(glow::FRONT);
                }
                Some(wgpu::Face::Back) => {
                    gl.enable(glow::CULL_FACE);
                    gl.cull_face(glow::BACK);
                }
                None => gl.disable(glow::CULL_FACE),
            }
        }
    }

    pub fn set_uniforms(
        &self,
        gl: &glow::Context,
        fog_color: &Vec4,
        blend_color: &Vec4,
        prim_color: &Vec4,
        env_color: &Vec4,
        key_center: &Vec3,
        key_scale: &Vec3,
        prim_lod: &Vec2,
        convert_k: &[i32; 6],
    ) {
        unsafe {
            if let Some(fog_color_location) = self.fog_color_location {
                gl.uniform_3_f32(
                    Some(&fog_color_location),
                    fog_color.x,
                    fog_color.y,
                    fog_color.z,
                );
            }

            if let Some(blend_color_location) = self.blend_color_location {
                gl.uniform_4_f32(
                    Some(&blend_color_location),
                    blend_color.x,
                    blend_color.y,
                    blend_color.z,
                    blend_color.w,
                );
            }

            if let Some(prim_color_location) = self.prim_color_location {
                gl.uniform_4_f32(
                    Some(&prim_color_location),
                    prim_color.x,
                    prim_color.y,
                    prim_color.z,
                    prim_color.w,
                );
            }

            if let Some(env_color_location) = self.env_color_location {
                gl.uniform_4_f32(
                    Some(&env_color_location),
                    env_color.x,
                    env_color.y,
                    env_color.z,
                    env_color.w,
                );
            }

            if let Some(key_center_location) = self.key_center_location {
                gl.uniform_3_f32(
                    Some(&key_center_location),
                    key_center.x,
                    key_center.y,
                    key_center.z,
                );
            }

            if let Some(key_scale_location) = self.key_scale_location {
                gl.uniform_3_f32(
                    Some(&key_scale_location),
                    key_scale.x,
                    key_scale.y,
                    key_scale.z,
                );
            }

            if let Some(prim_lod_frac_location) = self.prim_lod_frac_location {
                gl.uniform_1_f32(Some(&prim_lod_frac_location), prim_lod[0]);
            }

            if let Some(k4_location) = self.k4_location {
                gl.uniform_1_f32(Some(&k4_location), convert_k[4] as f32 / 255.0);
            }

            if let Some(k5_location) = self.k5_location {
                gl.uniform_1_f32(Some(&k5_location), convert_k[5] as f32 / 255.0);
            }
        }
    }

    pub fn draw_triangles(&self, gl: &glow::Context, buf_vbo: &[u8], buf_vbo_num_tris: usize) {
        unsafe {
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, buf_vbo, glow::STREAM_DRAW);
            gl.draw_arrays(glow::TRIANGLES, 0, buf_vbo_num_tris as i32 * 3);
        }
    }

    pub fn init(&self) {}

    pub fn on_resize(&self) {}

    pub fn start_frame(&mut self, gl: &glow::Context) {
        self.frame_count += 1;

        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.depth_mask(true);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.enable(glow::SCISSOR_TEST);
        }
    }

    pub fn end_frame(&self) {}

    pub fn finish_render(&self) {}
}
