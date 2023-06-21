use std::{collections::HashMap, borrow::Cow};

use bytemuck::{Pod, Zeroable};
use glam::Vec4Swizzles;
use log::{trace, warn};
use wgpu::util::{align_to, DeviceExt};

use crate::fast3d::{graphics::{GraphicsIntermediateFogParams, GraphicsIntermediateUniforms, GraphicsIntermediateTexture, GraphicsIntermediateSampler, GraphicsIntermediateStencil}, utils::color_combiner::CombineParams, gbi::defines::G_TX};

use super::wgpu_program::WgpuProgram;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
struct VertexUniforms {
    projection_matrix: [[f32; 4]; 4],
    _pad: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
struct VertexWithFogUniforms {
    projection_matrix: [[f32; 4]; 4],
    fog_multiplier: f32,
    fog_offset: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
struct FragmentBlendUniforms {
    blend_color: [f32; 4],
    fog_color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
struct FragmentCombineUniforms {
    prim_color: [f32; 4],
    env_color: [f32; 4],
    key_center: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _pad: u32,
    key_scale: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    __pad: u32,
    prim_lod_frac: f32,
    convert_k4: f32,
    convert_k5: f32,
    ___pad:  u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
struct FragmentFrameUniforms {
    count: u32,
    height: u32,
}

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
    depth_texture: wgpu::TextureView,

    vertex_buf: wgpu::Buffer,

    vertex_uniform_buf: wgpu::Buffer,
    vertex_bind_group_layout: wgpu::BindGroupLayout,
    vertex_bind_group: wgpu::BindGroup,

    blend_uniform_buf: wgpu::Buffer,
    combine_uniform_buf: wgpu::Buffer,
    frame_uniform_buf: wgpu::Buffer,
    fragment_uniform_bind_group_layout: wgpu::BindGroupLayout,
    fragment_uniform_bind_group: wgpu::BindGroup,

    pub shader_cache: HashMap<u64, WgpuProgram>,
    current_shader: u64,

    textures: Vec<TextureData>,
    active_texture: usize,
    current_texture_ids: [usize; 2],

    frame_count: i32,
    current_height: i32,
}

impl WgpuGraphicsDevice {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    fn create_depth_texture(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> wgpu::TextureView {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        });

        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_vertex_uniforms(device: &wgpu::Device) -> wgpu::Buffer {
        // Handle vertex uniform size ohne fog
        let vertex_uniform_size = std::mem::size_of::<VertexUniforms>() as wgpu::BufferAddress;
        let vertex_uniform_alignment = {
            let alignment = device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(vertex_uniform_size, alignment)
        };
        let vertex_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Uniform Buffer"),
            size: vertex_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        vertex_uniform_buf
    }

    fn create_fragment_uniforms(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Buffer) {
        // Handle blend uniform
        let blend_uniform_size = std::mem::size_of::<FragmentBlendUniforms>() as wgpu::BufferAddress;
        let blend_uniform_alignment = {
            let alignment = device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(blend_uniform_size, alignment)
        };
        let blend_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blend Uniform Buffer"),
            size: blend_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Handle combine uniform
        let combine_uniform_size = std::mem::size_of::<FragmentCombineUniforms>() as wgpu::BufferAddress;
        let combine_uniform_alignment = {
            let alignment = device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(combine_uniform_size, alignment)
        };
        let combine_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Combine Uniform Buffer"),
            size: combine_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Handle frame uniform
        let frame_uniform_size = std::mem::size_of::<FragmentFrameUniforms>() as wgpu::BufferAddress;
        let frame_uniform_alignment = {
            let alignment = device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(frame_uniform_size, alignment)
        };
        let frame_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame Uniform Buffer"),
            size: frame_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        (blend_uniform_buf, combine_uniform_buf, frame_uniform_buf)
    }

    fn create_vertex_bind_groups(device: &wgpu::Device, vertex_uniform_buf: &wgpu::Buffer) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let vertex_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vertex Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<VertexUniforms>() as wgpu::BufferAddress),
                },
                count: None,
            }],
        });

        let vertex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Bind Group"),
            layout: &vertex_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: vertex_uniform_buf,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<VertexUniforms>() as wgpu::BufferAddress),
                }),
            }],
        });

        (vertex_bind_group_layout, vertex_bind_group)
    }

    fn create_fragment_bind_groups(device: &wgpu::Device, blend_uniform_buf: &wgpu::Buffer, combine_uniform_buf: &wgpu::Buffer, frame_uniform_buf: &wgpu::Buffer) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let fragment_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Uniform Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<FragmentBlendUniforms>() as wgpu::BufferAddress),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<FragmentCombineUniforms>() as wgpu::BufferAddress),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<FragmentFrameUniforms>() as wgpu::BufferAddress),
                    },
                    count: None,
                }
            ],
        });

        let fragment_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Uniform Group"),
            layout: &fragment_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: blend_uniform_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<FragmentBlendUniforms>() as wgpu::BufferAddress),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: combine_uniform_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<FragmentCombineUniforms>() as wgpu::BufferAddress),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: frame_uniform_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<FragmentFrameUniforms>() as wgpu::BufferAddress),
                    }),
                }
            ],
        });

        (fragment_uniform_bind_group_layout, fragment_uniform_bind_group)
    }

    fn create_textures_bind_group(&self, device: &wgpu::Device, program: &WgpuProgram, texture_bind_group_layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
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
    pub fn new(config: &wgpu::SurfaceConfiguration, device: &wgpu::Device) -> Self {
        // Create the depth texture
        let depth_texture = Self::create_depth_texture(config, device);

        // Create vertex buffer
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 256 * 32 * 3 * ::std::mem::size_of::<f32>() as u64 * 50,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create the uniform buffers
        let vertex_uniform_buf = Self::create_vertex_uniforms(device);
        let (blend_uniform_buf, combine_uniform_buf, frame_uniform_buf) = Self::create_fragment_uniforms(device);

        // Create the bind groups
        let (vertex_bind_group_layout, vertex_bind_group) = Self::create_vertex_bind_groups(device, &vertex_uniform_buf); 
        let (fragment_uniform_bind_group_layout, fragment_uniform_bind_group) = Self::create_fragment_bind_groups(device, &blend_uniform_buf, &combine_uniform_buf, &frame_uniform_buf);

        Self {
            depth_texture,

            vertex_buf,

            vertex_uniform_buf,
            vertex_bind_group_layout,
            vertex_bind_group,

            blend_uniform_buf,
            combine_uniform_buf,
            frame_uniform_buf,
            fragment_uniform_bind_group_layout,
            fragment_uniform_bind_group,

            shader_cache: HashMap::new(),
            current_shader: 0,

            textures: Vec::new(),
            active_texture: 0,
            current_texture_ids: [0; 2],

            frame_count: 0,
            current_height: 0,
        }
    }

    pub fn resize(&mut self, config: &wgpu::SurfaceConfiguration, device: &wgpu::Device) {
        self.depth_texture = Self::create_depth_texture(config, device);
    }

    pub fn update_frame_count(&mut self) {
        self.frame_count += 1;
    }

    pub fn update_current_height(&mut self, height: i32) {
        self.current_height = height;
    }

    pub fn select_program(&mut self, device: &wgpu::Device, shader_hash: u64, other_mode_h: u32, other_mode_l: u32, geometry_mode: u32, combine: CombineParams) {
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
        let mut program = WgpuProgram::new(other_mode_h, other_mode_l, geometry_mode, combine);
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

    pub fn bind_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, tile: usize, texture: &mut GraphicsIntermediateTexture) {
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

    pub fn bind_sampler(&mut self, device: &wgpu::Device, tile: usize, sampler: &GraphicsIntermediateSampler) {
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

    pub fn update_uniforms(&mut self, queue: &wgpu::Queue, projection_matrix: glam::Mat4, fog: &GraphicsIntermediateFogParams, uniforms: &GraphicsIntermediateUniforms) {
        // Grab current program
        let program = self.shader_cache.get_mut(&self.current_shader).unwrap();

        // Update the vertex uniforms
        if program.get_define_bool("USE_FOG") {
            let uniform = VertexWithFogUniforms {
                projection_matrix: projection_matrix.transpose().to_cols_array_2d(),
                fog_multiplier: fog.multiplier as f32,
                fog_offset: fog.offset as f32,
                _pad: [0.0; 2],
            };

            queue.write_buffer(&self.vertex_uniform_buf, 0, bytemuck::bytes_of(&uniform));
        } else {
            let uniform = VertexUniforms {
                projection_matrix: projection_matrix.transpose().to_cols_array_2d(),
                _pad: [0.0; 4],
            };

            queue.write_buffer(&self.vertex_uniform_buf, 0, bytemuck::bytes_of(&uniform));
        }

        // Update the blend uniforms
        let uniform = FragmentBlendUniforms {
            blend_color: uniforms.blend.blend_color.to_array(),
            fog_color: uniforms.blend.fog_color.to_array(),
        };

        queue.write_buffer(&self.blend_uniform_buf, 0, bytemuck::bytes_of(&uniform));

        // Update the combine uniforms
        let uniform = FragmentCombineUniforms {
            prim_color: uniforms.combine.prim_color.to_array(),
            env_color: uniforms.combine.env_color.to_array(),
            key_center: uniforms.combine.key_center.to_array(),
            _pad: 0,
            key_scale: uniforms.combine.key_scale.to_array(),
            __pad: 0,
            prim_lod_frac: uniforms.combine.prim_lod.x,
            convert_k4: uniforms.combine.convert_k4,
            convert_k5: uniforms.combine.convert_k5,
            ___pad: 0,
        };

        queue.write_buffer(&self.combine_uniform_buf, 0, bytemuck::bytes_of(&uniform));

        // Update the frame uniforms
        let uniform = FragmentFrameUniforms {
            count: self.frame_count as u32,
            height: self.current_height as u32,
        };

        queue.write_buffer(&self.frame_uniform_buf, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn create_pipeline(&mut self, device: &wgpu::Device, surface_texture_format: wgpu::TextureFormat, blend_state: Option<wgpu::BlendState>, cull_mode: Option<wgpu::Face>, depth_stencil: Option<GraphicsIntermediateStencil>) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
        // Grab current program
        let program = self.shader_cache.get_mut(&self.current_shader).unwrap();

        // Create the texture bind group layout
        let texture_bind_group_layout = program.create_texture_bind_group_layout(device);

        // Create the pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&self.vertex_bind_group_layout, &self.fragment_uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create color target state
        let color_target_states = wgpu::ColorTargetState {
            format: surface_texture_format,
            blend: blend_state,
            write_mask: wgpu::ColorWrites::ALL,
        };

        // Depth stencil state
        let depth_stencil = depth_stencil.map(|ds| wgpu::DepthStencilState {
            format: Self::DEPTH_FORMAT,
            depth_write_enabled: ds.depth_write_enabled,
            depth_compare: ds.depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 0,
                slope_scale: if ds.polygon_offset { -2.0 } else { 0.0 },
                clamp: 0.0,
            },
        });

        // Create pipeline descriptor
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

        (texture_bind_group_layout, pipeline)
    }

    pub fn draw_triangles(&mut self, draw_call_index: usize, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, pipeline: &wgpu::RenderPipeline, texture_bind_group_layout: &wgpu::BindGroupLayout, viewport: &glam::Vec4, scissor: [u32; 4], buf_vbo: &[u8], num_tris: usize) {
        // Grab current program
        let program = self.shader_cache.get(&self.current_shader).unwrap();

        // Render the triangles
        encoder.push_debug_group(&format!("draw triangle pass: {}", draw_call_index));

        {
            // Create the texture bind groups
            let textures_bind_group = self.create_textures_bind_group(device, program, &texture_bind_group_layout);

            // Copy the vertex data to the buffer
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

            // queue.write_buffer(&self.vertex_buf, 0, buf_vbo);

            // Create the render pass
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("Game Render Pass: {}", draw_call_index)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            pass.push_debug_group("Prepare data for draw.");
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.vertex_bind_group, &[]);
            pass.set_bind_group(1, &self.fragment_uniform_bind_group, &[]);
            pass.set_bind_group(2, &textures_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            pass.set_viewport(viewport.x, viewport.y, viewport.z, viewport.w, 0.0, 1.0);
            pass.set_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
            pass.pop_debug_group();
            pass.insert_debug_marker("Draw!");
            pass.draw(0..(num_tris * 3) as u32, 0..1);
        }

        encoder.pop_debug_group();
    }
}
