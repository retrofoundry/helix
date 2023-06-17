use std::{
    borrow::Cow,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use wgpu::{util::DeviceExt, BindGroupLayout};

use crate::fast3d::{
    gbi::defines::G_TX,
    graphics::{
        GraphicsIntermediateSampler, GraphicsIntermediateStencil, GraphicsIntermediateTexture,
    },
    rdp::NUM_TILE_DESCRIPTORS,
    utils::{color_combiner::CombineParams, tile_descriptor::TileDescriptor},
};

use super::wgpu_program::WgpuProgram;

struct TextureData {
    texture_view: Option<wgpu::TextureView>,
    sampler: Option<wgpu::Sampler>,
}

impl TextureData {
    pub fn new() -> Self {
        Self {
            texture_view: None,
            sampler: None,
        }
    }
}

pub struct WgpuGraphicsDevice {
    viewport: glam::Vec4,
    scissor: [u32; 4],

    frame_count: i32,
    current_height: i32,

    shader_cache: HashMap<u64, WgpuProgram>,
    current_shader: u64,

    vertex_buf: wgpu::Buffer,
    blend_uniform_buf: wgpu::Buffer,
    combine_params_uniform_buf: wgpu::Buffer,
    frame_uniform_buf: wgpu::Buffer,

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
            size: 256 * 32 * 3 * 4 * 50,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Setup uniform buffers
        let blend_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blend Uniform Buffer"),
            size: (4 * 4) + (3 * 4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let combine_params_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Combine Params Uniform Buffer"),
            size: (4 * 4) + (4 * 4) + (3 * 4) + (3 * 4) + 4 + 4 + 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frame_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame Uniform Buffer"),
            size: 4 * 2,
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
                            min_binding_size: wgpu::BufferSize::new(28),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(68),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(8),
                        },
                        count: None,
                    },
                ],
            });

        Self {
            viewport: glam::Vec4::ZERO,
            scissor: [0; 4],

            frame_count: 0,
            current_height: 0,

            shader_cache: HashMap::new(),
            current_shader: 0,

            vertex_buf,
            blend_uniform_buf,
            combine_params_uniform_buf,
            frame_uniform_buf,

            uniform_bind_group_layout,

            textures: Vec::new(),
            active_texture: 0,
            current_texture_ids: [0; 2],
        }
    }

    fn compile_program(&self, device: &wgpu::Device, program: &mut WgpuProgram) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&program.processed_shader)),
        });

        program.compiled_program = Some(shader);
    }

    fn create_texture_bind_group_layout(
        &self,
        device: &wgpu::Device,
        program: &WgpuProgram,
    ) -> BindGroupLayout {
        {
            let mut group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();

            for i in 0..2 {
                let texture_index = format!("USE_TEXTURE{}", i);
                if program.get_define_bool(&texture_index) {
                    group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                        binding: i * 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    });

                    group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                        binding: (i * 2 + 1),
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // TODO: Is this the appropriate setting?
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    });
                }
            }

            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Textures/Samplers Group Layout"),
                entries: &group_layout_entries,
            })
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
                        self.textures[self.current_texture_ids[i as usize]]
                            .texture_view
                            .as_ref()
                            .unwrap(),
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
    }

    pub fn end_frame(&self) {}

    pub fn set_viewport(&mut self, viewport: glam::Vec4) {
        self.viewport = viewport;
        self.current_height = viewport.w as i32;
    }

    pub fn set_scissor(&mut self, scissor: [u32; 4]) {
        self.scissor = scissor;
    }

    pub fn load_program(
        &mut self,
        device: &wgpu::Device,
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
            self.current_shader = 0;
        }

        // check if the shader is in the cache
        if self.shader_cache.contains_key(&shader_hash) {
            self.current_shader = shader_hash;
            return;
        }

        // create the shader and add it to the cache
        let mut program = WgpuProgram::new(other_mode_h, other_mode_l, combine, tile_descriptors);
        program.init();
        program.preprocess();

        self.compile_program(device, &mut program);
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

        // Store the device texture
        let mut texture_data = TextureData::new();
        texture_data.texture_view =
            Some(device_texture.create_view(&wgpu::TextureViewDescriptor::default()));

        self.textures.push(texture_data);
        let texture_id = self.textures.len() as u32 - 1;

        texture.device_id = Some(texture_id);
        self.active_texture = tile;
        self.current_texture_ids[tile] = texture_id as usize;
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
        fog_color: &glam::Vec4,
        blend_color: &glam::Vec4,
        prim_color: &glam::Vec4,
        env_color: &glam::Vec4,
        key_center: &glam::Vec3,
        key_scale: &glam::Vec3,
        prim_lod: &glam::Vec2,
        convert_k: &[i32; 6],
    ) {
        // Update the Blend uniforms
        let blend_color_bytes = bytemuck::bytes_of(blend_color);
        let fog_color_sans_alpha = glam::Vec3::new(fog_color.x, fog_color.y, fog_color.z);
        let fog_color_bytes = bytemuck::bytes_of(&fog_color_sans_alpha);
        let blend_uniform_data_bytes: Vec<u8> = [blend_color_bytes, fog_color_bytes].concat();

        let staging_blend_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

        // Set the combine uniforms
        let prim_color_bytes = bytemuck::bytes_of(prim_color);
        let env_color_bytes = bytemuck::bytes_of(env_color);
        let key_center_bytes = bytemuck::bytes_of(key_center);
        let key_scale_bytes = bytemuck::bytes_of(key_scale);
        let prim_lod_bytes = bytemuck::bytes_of(&prim_lod.x);
        let convert_k4_bytes = bytemuck::bytes_of(&convert_k[4]);
        let convert_k5_bytes = bytemuck::bytes_of(&convert_k[5]);

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

        let staging_combine_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

        // Set the frame uniforms
        let frame_count_bytes = bytemuck::bytes_of(&self.frame_count);
        let frame_height_bytes = bytemuck::bytes_of(&self.current_height);

        let staging_frame_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging Frame Buffer"),
            contents: &[frame_count_bytes, frame_height_bytes].concat(),
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        encoder.copy_buffer_to_buffer(&staging_frame_buffer, 0, &self.frame_uniform_buf, 0, 8);
    }

    pub fn draw_triangles(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        surface_texture_format: wgpu::TextureFormat,
        blend_state: Option<wgpu::BlendState>,
        cull_mode: Option<wgpu::Face>,
        depth_stencil: Option<GraphicsIntermediateStencil>,
        buf_vbo: &[u8],
        buf_vbo_num_tris: usize,
    ) {
        let program = self.shader_cache.get(&self.current_shader).unwrap();

        // Setup texture/sampler group layout
        let texture_bind_group_layout = self.create_texture_bind_group_layout(device, program);

        // Setup pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&self.uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Describe vertex buffer layout
        let mut vertex_attributes = vec![
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0, // position
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 4 * 4, // color
                shader_location: 1,
            },
        ];

        if program.get_define_bool("USE_TEXTURE0") || program.get_define_bool("USE_TEXTURE1") {
            vertex_attributes.push(wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 4 * 8, // uv
                shader_location: 2,
            });
        }

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: program.vertex_size as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attributes,
        }];

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
                buffers: &vertex_buffers,
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
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        // Draw triangles
        rpass.push_debug_group("Prepare data for draw.");
        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, &uniform_bind_group, &[]);
        rpass.set_bind_group(1, &textures_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        rpass.set_viewport(
            self.viewport.x,
            self.viewport.y,
            self.viewport.z,
            self.viewport.w,
            0.0,
            1.0,
        );
        rpass.set_scissor_rect(
            self.scissor[0],
            self.scissor[1],
            self.scissor[2],
            self.scissor[3],
        );
        rpass.pop_debug_group();
        rpass.insert_debug_marker("Draw!");
        rpass.draw(0..buf_vbo_num_tris as u32, 0..1);
    }
}
