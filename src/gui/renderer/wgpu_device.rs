use std::{borrow::Cow, collections::HashMap};

use glam::Vec4Swizzles;
use log::warn;
use wgpu::{util::DeviceExt, BindGroupLayout, RenderPassDepthStencilAttachment};

use crate::fast3d::{
    gbi::defines::G_TX,
    graphics::{
        GraphicsIntermediateSampler, GraphicsIntermediateStencil, GraphicsIntermediateTexture,
        GraphicsIntermediateUniforms, GraphicsIntermediateUniformsBlend,
        GraphicsIntermediateUniformsCombine,
    },
    utils::color_combiner::CombineParams,
};

use super::wgpu_program::WgpuProgram;

struct TextureData {
    texture_view: wgpu::TextureView,
    sampler: Option<wgpu::Sampler>,
}

impl TextureData {
    pub fn new(texture_view: wgpu::TextureView) -> Self {
        Self {
            texture_view,
            sampler: None,
        }
    }
}

pub struct WgpuGraphicsDevice {
    frame_count: i32,
    current_height: i32,

    shader_cache: HashMap<u64, WgpuProgram>,
    current_shader: u64,

    vertex_buf: wgpu::Buffer,
    blend_uniform_buf: wgpu::Buffer,
    combine_params_uniform_buf: wgpu::Buffer,
    frame_uniform_buf: wgpu::Buffer,

    last_blend_uniform: GraphicsIntermediateUniformsBlend,
    last_combine_uniform: GraphicsIntermediateUniformsCombine,
    has_set_frame_uniform_buf: bool,

    uniform_bind_group_layout: wgpu::BindGroupLayout,

    textures: Vec<TextureData>,
    active_texture: usize,
    current_texture_ids: [usize; 2],
}

impl WgpuGraphicsDevice {
    pub fn new(device: &wgpu::Device) -> Self {
        // Setup vertex buffer
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 256 * 32 * 3 * ::std::mem::size_of::<f32>() as u64 * 50,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Setup uniform buffers
        let blend_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blend Uniform Buffer"),
            size: 8 * ::std::mem::size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let combine_params_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Combine Params Uniform Buffer"),
            size: 20 * ::std::mem::size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frame_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame Uniform Buffer"),
            size: 2 * ::std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Setup uniform bind group layout
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniforms Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        Self {
            frame_count: 0,
            current_height: 0,

            shader_cache: HashMap::new(),
            current_shader: 0,

            vertex_buf,
            blend_uniform_buf,
            combine_params_uniform_buf,
            frame_uniform_buf,

            last_blend_uniform: GraphicsIntermediateUniformsBlend::EMPTY,
            last_combine_uniform: GraphicsIntermediateUniformsCombine::EMPTY,
            has_set_frame_uniform_buf: false,

            uniform_bind_group_layout,

            textures: Vec::new(),
            active_texture: 0,
            current_texture_ids: [0; 2],
        }
    }

    fn create_uniform_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniforms Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.blend_uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.combine_params_uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.frame_uniform_buf.as_entire_binding(),
                },
            ],
        })
    }

    fn create_textures_bind_group(
        &self,
        device: &wgpu::Device,
        program: &WgpuProgram,
        texture_bind_group_layout: &BindGroupLayout,
    ) -> wgpu::BindGroup {
        let mut texture_bind_group_entries: Vec<wgpu::BindGroupEntry> = Vec::new();

        for i in 0..2 {
            let texture_index = format!("USE_TEXTURE{}", i);
            if program.get_define_bool(&texture_index) {
                texture_bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: i * 2,
                    resource: wgpu::BindingResource::TextureView(
                        &self.textures[self.current_texture_ids[i as usize]].texture_view,
                    ),
                });

                texture_bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: (i * 2 + 1),
                    resource: wgpu::BindingResource::Sampler(
                        self.textures[self.current_texture_ids[i as usize]]
                            .sampler
                            .as_ref()
                            .unwrap(),
                    ),
                });
            }
        }

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &texture_bind_group_entries,
            label: None,
        })
    }

    fn gfx_cm_to_wgpu(val: u32) -> wgpu::AddressMode {
        if val & G_TX::CLAMP as u32 != 0 {
            return wgpu::AddressMode::ClampToEdge;
        }

        if val & G_TX::MIRROR as u32 != 0 {
            return wgpu::AddressMode::MirrorRepeat;
        }

        wgpu::AddressMode::Repeat
    }
}

impl WgpuGraphicsDevice {
    // MARK: - Public API

    pub fn start_frame(&mut self) {
        self.frame_count += 1;
        self.has_set_frame_uniform_buf = false;
    }

    pub fn end_frame(&self) {}

    // pub fn set_viewport(&mut self, viewport: glam::Vec4) {
    //     self.viewport = viewport;
    //     self.current_height = viewport.w as i32;
    // }

    // pub fn set_scissor(&mut self, scissor: [u32; 4]) {
    //     self.scissor = scissor;
    // }

    pub fn load_program(
        &mut self,
        device: &wgpu::Device,
        shader_hash: u64,
        other_mode_h: u32,
        other_mode_l: u32,
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
        let mut program = WgpuProgram::new(other_mode_h, other_mode_l, combine);
        program.init();
        program.preprocess();

        program.compiled_program =
            Some(device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&program.processed_shader)),
            }));

        self.current_shader = shader_hash;
        self.shader_cache.insert(shader_hash, program);
    }

    pub fn bind_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tile: usize,
        texture: &mut GraphicsIntermediateTexture,
    ) {
        // check if we've already uploaded this texture to the GPU
        if let Some(texture_id) = texture.device_id {
            self.active_texture = tile;
            self.current_texture_ids[tile] = texture_id as usize;

            return;
        }

        // Create device texture
        let texture_extent = wgpu::Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        };

        let device_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Write data to the device texture
        let bytes_per_pixel = 4;
        let bytes_per_row = bytes_per_pixel * texture.width;

        queue.write_texture(
            device_texture.as_image_copy(),
            &texture.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
            texture_extent,
        );

        // Create the texture
        let texture_view = device_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.active_texture = tile;
        self.current_texture_ids[tile] = self.textures.len();
        texture.device_id = Some(self.textures.len() as u32);

        self.textures.push(TextureData::new(texture_view));
    }

    pub fn bind_sampler(
        &mut self,
        device: &wgpu::Device,
        tile: usize,
        sampler: &GraphicsIntermediateSampler,
    ) {
        if let Some(texture_data) = self.textures.get_mut(self.current_texture_ids[tile]) {
            texture_data.sampler = Some(device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: Self::gfx_cm_to_wgpu(sampler.clamp_s),
                address_mode_v: Self::gfx_cm_to_wgpu(sampler.clamp_t),
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: if sampler.linear_filter {
                    wgpu::FilterMode::Linear
                } else {
                    wgpu::FilterMode::Nearest
                },
                min_filter: if sampler.linear_filter {
                    wgpu::FilterMode::Linear
                } else {
                    wgpu::FilterMode::Nearest
                },
                ..Default::default()
            }));
        }
    }

    pub fn set_uniforms(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        uniforms: &GraphicsIntermediateUniforms,
    ) {
        // Update the Blend uniforms
        if self.last_blend_uniform != uniforms.blend {
            warn!("Updating blend uniforms");
            let blend_color_bytes = bytemuck::bytes_of(&uniforms.blend.blend_color);
            let fog_color_sans_alpha = uniforms.blend.fog_color.xyz();
            let fog_color_bytes = bytemuck::bytes_of(&fog_color_sans_alpha);
            let blend_uniform_data_bytes: Vec<u8> = [blend_color_bytes, fog_color_bytes].concat();

            let staging_blend_buffer =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Staging Blend Buffer"),
                    contents: &blend_uniform_data_bytes,
                    usage: wgpu::BufferUsages::COPY_SRC,
                });

            encoder.copy_buffer_to_buffer(
                &staging_blend_buffer,
                0,
                &self.blend_uniform_buf,
                0,
                blend_uniform_data_bytes.len() as u64,
            );

            self.last_blend_uniform = uniforms.blend;
        }

        // Set the combine uniforms
        if self.last_combine_uniform != uniforms.combine {
            warn!("Updating combine uniforms");
            let prim_color_bytes = bytemuck::bytes_of(&uniforms.combine.prim_color);
            let env_color_bytes = bytemuck::bytes_of(&uniforms.combine.env_color);
            let key_center_bytes = bytemuck::bytes_of(&uniforms.combine.key_center);
            let key_scale_bytes = bytemuck::bytes_of(&uniforms.combine.key_scale);
            let prim_lod_bytes = bytemuck::bytes_of(&uniforms.combine.prim_lod.x);
            let convert_k4_bytes = bytemuck::bytes_of(&uniforms.combine.convert_k4);
            let convert_k5_bytes = bytemuck::bytes_of(&uniforms.combine.convert_k5);

            let combine_uniform_data_bytes: Vec<u8> = [
                prim_color_bytes,
                env_color_bytes,
                key_center_bytes,
                key_scale_bytes,
                prim_lod_bytes,
                convert_k4_bytes,
                convert_k5_bytes,
            ]
            .concat();

            let staging_combine_buffer =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Staging Combine Buffer"),
                    contents: &combine_uniform_data_bytes,
                    usage: wgpu::BufferUsages::COPY_SRC,
                });

            encoder.copy_buffer_to_buffer(
                &staging_combine_buffer,
                0,
                &self.combine_params_uniform_buf,
                0,
                combine_uniform_data_bytes.len() as u64,
            );

            self.last_combine_uniform = uniforms.combine;
        }

        // Set the frame uniforms
        if !self.has_set_frame_uniform_buf {
            warn!("Updating frame uniforms");
            let frame_count_bytes = bytemuck::bytes_of(&self.frame_count);
            let frame_height_bytes = bytemuck::bytes_of(&self.current_height);

            let staging_frame_buffer =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Staging Frame Buffer"),
                    contents: &[frame_count_bytes, frame_height_bytes].concat(),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });

            encoder.copy_buffer_to_buffer(&staging_frame_buffer, 0, &self.frame_uniform_buf, 0, 8);
            self.has_set_frame_uniform_buf = true;
        }
    }

    pub fn draw_triangles(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_texture: &wgpu::TextureView,
        surface_texture_format: wgpu::TextureFormat,
        viewport: &glam::Vec4,
        scissor: [u32; 4],
        blend_state: Option<wgpu::BlendState>,
        cull_mode: Option<wgpu::Face>,
        depth_stencil: Option<GraphicsIntermediateStencil>,
        buf_vbo: &[u8],
        buf_vbo_num_tris: usize,
    ) {
        self.current_height = viewport.w as i32;

        let program = self.shader_cache.get(&self.current_shader).unwrap();

        // Setup texture/sampler group layout
        let texture_bind_group_layout = program.create_texture_bind_group_layout(device);

        // Setup pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&self.uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Bind groups
        let uniform_bind_group = self.create_uniform_bind_group(device);
        let textures_bind_group =
            self.create_textures_bind_group(device, program, &texture_bind_group_layout);

        // Create color target state
        let color_target_states = wgpu::ColorTargetState {
            format: surface_texture_format,
            blend: blend_state,
            write_mask: wgpu::ColorWrites::ALL,
        };

        // Depth stencil state
        let depth_stencil = depth_stencil.map(|ds| wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: ds.depth_write_enabled,
            depth_compare: ds.depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 0,
                slope_scale: if ds.polygon_offset { -2.0 } else { 0.0 },
                clamp: 0.0,
            },
        });

        // Setup Pipeline Descriptor
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: program.compiled_program.as_ref().unwrap(),
                entry_point: "vs_main",
                buffers: &[program.vertex_description()],
            },
            fragment: Some(wgpu::FragmentState {
                module: program.compiled_program.as_ref().unwrap(),
                entry_point: "fs_main",
                targets: &[Some(color_target_states)],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let staging_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Staging Buffer"),
            contents: buf_vbo,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        encoder.copy_buffer_to_buffer(
            &staging_vertex_buffer,
            0,
            &self.vertex_buf,
            0,
            buf_vbo.len() as u64,
        );

        // Create rpass
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Game Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        // Draw triangles
        rpass.push_debug_group("Prepare data for draw.");
        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, &uniform_bind_group, &[]);
        rpass.set_bind_group(1, &textures_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        rpass.set_viewport(viewport.x, viewport.y, viewport.z, viewport.w, 0.0, 1.0);
        rpass.set_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
        rpass.pop_debug_group();
        rpass.insert_debug_marker("Draw!");
        rpass.draw(0..(buf_vbo_num_tris * 3) as u32, 0..1);
    }
}
