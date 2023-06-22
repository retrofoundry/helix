use std::{borrow::Cow, collections::HashMap};

use glam::Vec4Swizzles;
use glium::{
    draw_parameters::{DepthClamp, PolygonOffset},
    index::{NoIndices, PrimitiveType},
    program::ProgramCreationInput,
    texture::{RawImage2d, Texture2d},
    uniforms::{
        MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior, SamplerWrapFunction,
        UniformValue, Uniforms,
    },
    vertex::{AttributeType, VertexBufferAny},
    BackfaceCullingMode, BlendingFunction, DepthTest, Display, DrawParameters, Frame,
    LinearBlendingFactor, Program, Surface,
};

use crate::fast3d::{
    gbi::defines::G_TX,
    graphics::{
        GraphicsIntermediateFogParams, GraphicsIntermediateSampler, GraphicsIntermediateStencil,
        GraphicsIntermediateTexture, GraphicsIntermediateUniforms,
    },
    utils::color_combiner::CombineParams,
};

use super::opengl_program::OpenGLProgram;

struct TextureData {
    texture: Texture2d,
    sampler: Option<SamplerBehavior>,
}

impl TextureData {
    pub fn new(texture: Texture2d) -> Self {
        Self {
            texture,
            sampler: None,
        }
    }
}

#[derive(Default)]
struct UniformVec<'a, 'b> {
    pub uniforms: Vec<(&'a str, UniformValue<'b>)>,
}

impl Uniforms for UniformVec<'_, '_> {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut func: F) {
        for uniform in &self.uniforms {
            func(uniform.0, uniform.1);
        }
    }
}

pub struct GliumGraphicsDevice<'draw> {
    pub shader_cache: HashMap<u64, OpenGLProgram<Program>>,
    current_shader: u64,

    textures: Vec<TextureData>,
    active_texture: usize,
    current_texture_ids: [usize; 2],

    frame_count: i32,
    current_height: i32,

    draw_params: DrawParameters<'draw>,
}

fn blend_component_to_glium(component: wgpu::BlendComponent) -> BlendingFunction {
    match component.operation {
        wgpu::BlendOperation::Add => BlendingFunction::Addition {
            source: blend_factor_to_glium(component.src_factor),
            destination: blend_factor_to_glium(component.dst_factor),
        },
        wgpu::BlendOperation::Subtract => BlendingFunction::Subtraction {
            source: blend_factor_to_glium(component.src_factor),
            destination: blend_factor_to_glium(component.dst_factor),
        },
        wgpu::BlendOperation::ReverseSubtract => BlendingFunction::ReverseSubtraction {
            source: blend_factor_to_glium(component.src_factor),
            destination: blend_factor_to_glium(component.dst_factor),
        },
        wgpu::BlendOperation::Min => BlendingFunction::Min,
        wgpu::BlendOperation::Max => BlendingFunction::Max,
    }
}

fn blend_factor_to_glium(factor: wgpu::BlendFactor) -> LinearBlendingFactor {
    match factor {
        wgpu::BlendFactor::Zero => LinearBlendingFactor::Zero,
        wgpu::BlendFactor::One => LinearBlendingFactor::One,
        wgpu::BlendFactor::Src => LinearBlendingFactor::SourceColor,
        wgpu::BlendFactor::OneMinusSrc => LinearBlendingFactor::OneMinusSourceColor,
        wgpu::BlendFactor::SrcAlpha => LinearBlendingFactor::SourceAlpha,
        wgpu::BlendFactor::OneMinusSrcAlpha => LinearBlendingFactor::OneMinusSourceAlpha,
        wgpu::BlendFactor::Dst => LinearBlendingFactor::DestinationColor,
        wgpu::BlendFactor::OneMinusDst => LinearBlendingFactor::OneMinusDestinationColor,
        wgpu::BlendFactor::DstAlpha => LinearBlendingFactor::DestinationAlpha,
        wgpu::BlendFactor::OneMinusDstAlpha => LinearBlendingFactor::OneMinusDestinationAlpha,
        wgpu::BlendFactor::SrcAlphaSaturated => LinearBlendingFactor::SourceAlphaSaturate,
        wgpu::BlendFactor::Constant => LinearBlendingFactor::ConstantColor,
        wgpu::BlendFactor::OneMinusConstant => LinearBlendingFactor::OneMinusConstantColor,
    }
}

fn clamp_to_glium(clamp: u32) -> SamplerWrapFunction {
    if clamp & G_TX::CLAMP as u32 != 0 {
        return SamplerWrapFunction::Clamp;
    }

    if clamp & G_TX::MIRROR as u32 != 0 {
        return SamplerWrapFunction::Mirror;
    }

    SamplerWrapFunction::Repeat
}

impl<'draw> Default for GliumGraphicsDevice<'draw> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'draw> GliumGraphicsDevice<'draw> {
    pub fn new() -> Self {
        Self {
            shader_cache: HashMap::new(),
            current_shader: 0,

            textures: Vec::new(),
            active_texture: 0,
            current_texture_ids: [0; 2],

            frame_count: 0,
            current_height: 0,

            draw_params: DrawParameters {
                ..Default::default()
            },
        }
    }

    pub fn start_frame(&mut self, target: &mut Frame) {
        self.frame_count += 1;

        target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        self.draw_params = DrawParameters {
            ..Default::default()
        };
    }

    pub fn end_frame(&self) {}

    pub fn set_cull_mode(&mut self, cull_mode: Option<wgpu::Face>) {
        self.draw_params.backface_culling = match cull_mode {
            Some(wgpu::Face::Front) => BackfaceCullingMode::CullCounterClockwise,
            Some(wgpu::Face::Back) => BackfaceCullingMode::CullClockwise,
            None => BackfaceCullingMode::CullingDisabled,
        }
    }

    pub fn set_depth_stencil_params(&mut self, params: Option<GraphicsIntermediateStencil>) {
        self.draw_params.depth = if let Some(params) = params {
            glium::Depth {
                test: match params.depth_compare {
                    wgpu::CompareFunction::Never => DepthTest::Ignore,
                    wgpu::CompareFunction::Less => DepthTest::IfLess,
                    wgpu::CompareFunction::Equal => DepthTest::IfEqual,
                    wgpu::CompareFunction::LessEqual => DepthTest::IfLessOrEqual,
                    wgpu::CompareFunction::Greater => DepthTest::IfMore,
                    wgpu::CompareFunction::NotEqual => DepthTest::IfNotEqual,
                    wgpu::CompareFunction::GreaterEqual => DepthTest::IfMoreOrEqual,
                    wgpu::CompareFunction::Always => DepthTest::Overwrite,
                },
                write: params.depth_write_enabled,
                clamp: DepthClamp::Clamp,
                ..Default::default()
            }
        } else {
            glium::Depth {
                clamp: DepthClamp::Clamp,
                ..Default::default()
            }
        };

        self.draw_params.polygon_offset = if let Some(params) = params {
            PolygonOffset {
                factor: if params.polygon_offset { -2.0 } else { 0.0 },
                units: if params.polygon_offset { 2.0 } else { 0.0 },
                fill: true,
                ..Default::default()
            }
        } else {
            PolygonOffset {
                ..Default::default()
            }
        };
    }

    pub fn set_blend_state(&mut self, blend_state: Option<wgpu::BlendState>) {
        self.draw_params.blend = if let Some(blend_state) = blend_state {
            glium::Blend {
                color: blend_component_to_glium(blend_state.color),
                alpha: blend_component_to_glium(blend_state.alpha),
                ..Default::default()
            }
        } else {
            glium::Blend {
                ..Default::default()
            }
        };
    }

    pub fn set_viewport(&mut self, viewport: &glam::Vec4) {
        self.draw_params.viewport = Some(glium::Rect {
            left: viewport.x as u32,
            bottom: viewport.y as u32,
            width: viewport.z as u32,
            height: viewport.w as u32,
        });

        self.current_height = viewport.w as i32;
    }

    pub fn set_scissor(&mut self, scissor: [u32; 4]) {
        self.draw_params.scissor = Some(glium::Rect {
            left: scissor[0],
            bottom: scissor[1],
            width: scissor[2],
            height: scissor[3],
        });
    }

    pub fn load_program(
        &mut self,
        display: &Display,
        shader_hash: u64,
        other_mode_h: u32,
        other_mode_l: u32,
        geometry_mode: u32,
        combine: CombineParams,
    ) {
        // check if the shader is already loaded
        if self.current_shader == shader_hash {
            return;
        }

        // unload the current shader
        if self.current_shader != 0 {
            self.current_shader = 0;
        }

        // check if the shader is in the cache
        if self.shader_cache.contains_key(&shader_hash) {
            self.current_shader = shader_hash;
            return;
        }

        // create the shader and add it to the cache
        let mut program = OpenGLProgram::new(other_mode_h, other_mode_l, geometry_mode, combine);
        program.init();
        program.preprocess();

        let source = ProgramCreationInput::SourceCode {
            vertex_shader: &program.preprocessed_vertex,
            fragment_shader: &program.preprocessed_frag,
            geometry_shader: None,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb: true, // workaround to avoid glium doing gamma correction
            uses_point_size: false,
        };

        program.compiled_program = Some(Program::new(display, source).unwrap());

        self.current_shader = shader_hash;
        self.shader_cache.insert(shader_hash, program);
    }

    pub fn bind_texture(
        &mut self,
        display: &Display,
        tile: usize,
        texture: &mut GraphicsIntermediateTexture,
    ) {
        // check if we've already uploaded this texture to the GPU
        if let Some(texture_id) = texture.device_id {
            // trace!("Texture found in GPU cache");
            self.active_texture = tile;
            self.current_texture_ids[tile] = texture_id as usize;

            return;
        }

        // Create the texture
        let raw_texture =
            RawImage2d::from_raw_rgba(texture.data.clone(), (texture.width, texture.height));
        let native_texture = Texture2d::new(display, raw_texture).unwrap();

        self.active_texture = tile;
        self.current_texture_ids[tile] = self.textures.len();
        texture.device_id = Some(self.textures.len() as u32);

        self.textures.push(TextureData::new(native_texture));
    }

    pub fn bind_sampler(&mut self, tile: usize, sampler: &GraphicsIntermediateSampler) {
        if let Some(texture_data) = self.textures.get_mut(self.current_texture_ids[tile]) {
            let wrap_s = clamp_to_glium(sampler.clamp_s);
            let wrap_t = clamp_to_glium(sampler.clamp_t);

            let native_sampler = SamplerBehavior {
                minify_filter: if sampler.linear_filter {
                    MinifySamplerFilter::Linear
                } else {
                    MinifySamplerFilter::Nearest
                },
                magnify_filter: if sampler.linear_filter {
                    MagnifySamplerFilter::Linear
                } else {
                    MagnifySamplerFilter::Nearest
                },
                wrap_function: (wrap_s, wrap_t, SamplerWrapFunction::Repeat),
                ..Default::default()
            };

            texture_data.sampler = Some(native_sampler);
        }
    }

    pub fn draw_triangles(
        &self,
        display: &Display,
        target: &mut Frame,
        projection_matrix: glam::Mat4,
        fog: &GraphicsIntermediateFogParams,
        vbo: &[u8],
        uniforms: &GraphicsIntermediateUniforms,
    ) {
        // Grab current program
        let program = self.shader_cache.get(&self.current_shader).unwrap();

        // Setup vertex buffer
        let mut num_floats = 8;
        let mut vertex_format_data = vec![
            (
                Cow::Borrowed("aVtxPos"),
                0,
                -1,
                AttributeType::F32F32F32F32,
                false,
            ),
            (
                Cow::Borrowed("aVtxColor"),
                4 * ::std::mem::size_of::<f32>(),
                -1,
                AttributeType::F32F32F32F32,
                false,
            ),
        ];

        if program.get_define_bool("USE_TEXTURE0") || program.get_define_bool("USE_TEXTURE1") {
            num_floats += 2;
            vertex_format_data.push((
                Cow::Borrowed("aTexCoord"),
                8 * ::std::mem::size_of::<f32>(),
                -1,
                AttributeType::F32F32,
                false,
            ));
        }

        let vertex_buffer = unsafe {
            VertexBufferAny::new_raw(
                display,
                vbo,
                Cow::Owned(vertex_format_data),
                num_floats * ::std::mem::size_of::<f32>(),
            )
        }
        .unwrap();

        // Setup uniforms
        let mut shader_uniforms = vec![
            (
                "uProjection",
                UniformValue::Mat4(projection_matrix.to_cols_array_2d()),
            ),
            (
                "uBlendColor",
                UniformValue::Vec4(uniforms.blend.blend_color.to_array()),
            ),
            (
                "uPrimColor",
                UniformValue::Vec4(uniforms.combine.prim_color.to_array()),
            ),
            (
                "uEnvColor",
                UniformValue::Vec4(uniforms.combine.env_color.to_array()),
            ),
            (
                "uKeyCenter",
                UniformValue::Vec3(uniforms.combine.key_center.to_array()),
            ),
            (
                "uKeyScale",
                UniformValue::Vec3(uniforms.combine.key_scale.to_array()),
            ),
            ("uPrimLOD", UniformValue::Float(uniforms.combine.prim_lod.x)),
            ("uK4", UniformValue::Float(uniforms.combine.convert_k4)),
            ("uK5", UniformValue::Float(uniforms.combine.convert_k5)),
        ];

        if program.get_define_bool("USE_FOG") {
            shader_uniforms.push((
                "uFogColor",
                UniformValue::Vec3(uniforms.blend.fog_color.xyz().to_array()),
            ));

            shader_uniforms.push(("uFogMultiplier", UniformValue::Float(fog.multiplier as f32)));
            shader_uniforms.push(("uFogOffset", UniformValue::Float(fog.offset as f32)));
        }

        if program.get_define_bool("USE_TEXTURE0") {
            let texture = self.textures.get(self.current_texture_ids[0]).unwrap();
            shader_uniforms.push((
                "uTex0",
                UniformValue::Texture2d(&texture.texture, texture.sampler),
            ));
        }

        if program.get_define_bool("USE_TEXTURE1") {
            let texture = self.textures.get(self.current_texture_ids[1]).unwrap();
            shader_uniforms.push((
                "uTex1",
                UniformValue::Texture2d(&texture.texture, texture.sampler),
            ));
        }

        if program.get_define_bool("USE_ALPHA") && program.get_define_bool("ALPHA_COMPARE_DITHER") {
            shader_uniforms.push(("uFrameCount", UniformValue::SignedInt(self.frame_count)));
            shader_uniforms.push(("uFrameHeight", UniformValue::SignedInt(self.current_height)));
        }

        // Draw triangles
        target
            .draw(
                &vertex_buffer,
                NoIndices(PrimitiveType::TrianglesList),
                program.compiled_program.as_ref().unwrap(),
                &UniformVec {
                    uniforms: shader_uniforms,
                },
                &self.draw_params,
            )
            .unwrap();
    }
}
