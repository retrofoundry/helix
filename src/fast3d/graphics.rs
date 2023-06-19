use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}};

use super::{
    rdp::NUM_TILE_DESCRIPTORS,
    utils::{
        color_combiner::CombineParams,
        texture::{ImageFormat, ImageSize},
        texture_cache::TextureCache,
        tile_descriptor::TileDescriptor,
    },
};

const TEXTURE_CACHE_MAX_SIZE: usize = 500;

pub struct GraphicsIntermediateTexture {
    pub game_address: usize,
    pub format: ImageFormat,
    pub size: ImageSize,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,

    // when a texture has been created in a gfx backend, this field will be Some
    pub device_id: Option<u32>,
}

impl GraphicsIntermediateTexture {
    pub fn new(
        game_address: usize,
        format: ImageFormat,
        size: ImageSize,
        width: u32,
        height: u32,
        data: Vec<u8>,
    ) -> Self {
        Self {
            game_address,
            format,
            size,
            width,
            height,
            data,
            device_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsIntermediateSampler {
    pub tile: usize,
    pub linear_filter: bool,
    pub clamp_s: u32,
    pub clamp_t: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsIntermediateStencil {
    pub depth_write_enabled: bool,
    pub depth_compare: wgpu::CompareFunction,
    pub polygon_offset: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsIntermediateUniforms {
    pub fog_color: glam::Vec4,
    pub blend_color: glam::Vec4,
    pub prim_color: glam::Vec4,
    pub env_color: glam::Vec4,
    pub key_center: glam::Vec3,
    pub key_scale: glam::Vec3,
    pub prim_lod: glam::Vec2,
    pub convert_k: [i32; 6],
}

impl GraphicsIntermediateUniforms {
    pub const EMPTY: Self = GraphicsIntermediateUniforms {
        fog_color: glam::Vec4::ZERO,
        blend_color: glam::Vec4::ZERO,
        prim_color: glam::Vec4::ZERO,
        env_color: glam::Vec4::ZERO,
        key_center: glam::Vec3::ZERO,
        key_scale: glam::Vec3::ZERO,
        prim_lod: glam::Vec2::ZERO,
        convert_k: [0; 6],
    };
}

#[derive(Debug, Clone)]
pub struct GraphicsIntermediateVBO {
    pub vbo: Vec<u8>,
    pub num_tris: usize,
}

impl GraphicsIntermediateVBO {
    pub const EMPTY: Self = GraphicsIntermediateVBO {
        vbo: Vec::new(),
        num_tris: 0,
    };
}

#[derive(Debug, Clone)]
pub struct GraphicsDrawCall {
    // Shader Configuration
    pub other_mode_h: u32,
    pub other_mode_l: u32,
    pub combine: CombineParams,
    pub tile_descriptors: [TileDescriptor; NUM_TILE_DESCRIPTORS],
    pub shader_hash: u64,

    // Textures
    pub textures: [Option<u64>; 2],

    // Samplers
    pub samplers: [Option<GraphicsIntermediateSampler>; 2],

    // Stencil
    pub stencil: Option<GraphicsIntermediateStencil>,

    // Viewport
    pub viewport: glam::Vec4,

    // Scissor
    pub scissor: [u32; 4],

    // Blend State
    pub blend_state: Option<wgpu::BlendState>,

    // Cull Mode
    pub cull_mode: Option<wgpu::Face>,

    // Uniforms
    pub uniforms: GraphicsIntermediateUniforms,

    // Triangle Data
    pub vbo: GraphicsIntermediateVBO,
}

impl GraphicsDrawCall {
    pub const EMPTY: Self = GraphicsDrawCall {
        other_mode_h: 0,
        other_mode_l: 0,
        combine: CombineParams::ZERO,
        tile_descriptors: [TileDescriptor::EMPTY; NUM_TILE_DESCRIPTORS],
        shader_hash: 0,
        textures: [None; 2],
        samplers: [None; 2],
        stencil: None,
        viewport: glam::Vec4::ZERO,
        scissor: [0; 4],
        blend_state: None,
        cull_mode: None,
        uniforms: GraphicsIntermediateUniforms::EMPTY,
        vbo: GraphicsIntermediateVBO::EMPTY,
    };

    pub fn finalize(&mut self) {
        // compute the shader hash and store it
        let mut hasher = DefaultHasher::new();

        self.other_mode_h.hash(&mut hasher);
        self.other_mode_l.hash(&mut hasher);
        self.combine.hash(&mut hasher);

        self.shader_hash = hasher.finish();
    }
}

pub struct GraphicsIntermediateDevice {
    pub texture_cache: TextureCache,
    pub draw_calls: Vec<GraphicsDrawCall>,
}

impl GraphicsIntermediateDevice {
    pub fn new() -> Self {
        GraphicsIntermediateDevice {
            texture_cache: TextureCache::new(TEXTURE_CACHE_MAX_SIZE),
            // start draw calls with a default draw call
            draw_calls: vec![GraphicsDrawCall::EMPTY],
        }
    }

    fn current_draw_call(&mut self) -> &mut GraphicsDrawCall {
        self.draw_calls.last_mut().unwrap()
    }

    fn new_draw_call(&mut self) {
        let draw_call = self.current_draw_call();
        let draw_call = draw_call.clone();
        self.draw_calls.push(draw_call);
    }

    // Public API

    pub fn clear_draw_calls(&mut self) {
        let draw_call = self.current_draw_call();
        let draw_call = draw_call.clone();
        self.draw_calls = vec![draw_call];
    }

    pub fn clear_textures(&mut self, index: usize) {
        let draw_call = self.current_draw_call();
        draw_call.textures[index] = None;
    }

    pub fn is_z_from_0_to_1(&self) -> bool {
        // false for OpenGL, true for WGPU
        false
    }

    pub fn set_program_params(
        &mut self,
        other_mode_h: u32,
        other_mode_l: u32,
        combine: CombineParams,
        tile_descriptors: [TileDescriptor; NUM_TILE_DESCRIPTORS],
    ) {
        let draw_call = self.current_draw_call();
        draw_call.other_mode_h = other_mode_h;
        draw_call.other_mode_l = other_mode_l;
        draw_call.combine = combine;
        draw_call.tile_descriptors = tile_descriptors;
    }

    pub fn set_texture(&mut self, tile: usize, hash: u64) {
        let draw_call = self.current_draw_call();
        draw_call.textures[tile] = Some(hash);
    }

    pub fn set_sampler_parameters(
        &mut self,
        tile: usize,
        linear_filter: bool,
        clamp_s: u32,
        clamp_t: u32,
    ) {
        let draw_call = self.current_draw_call();
        draw_call.samplers[tile] = Some(GraphicsIntermediateSampler {
            tile,
            linear_filter,
            clamp_s,
            clamp_t,
        });
    }

    pub fn set_depth_stencil_params(
        &mut self,
        _depth_test_enabled: bool,
        depth_write_enabled: bool,
        depth_compare: wgpu::CompareFunction,
        polygon_offset: bool,
    ) {
        let draw_call = self.current_draw_call();
        draw_call.stencil = Some(GraphicsIntermediateStencil {
            depth_write_enabled,
            depth_compare,
            polygon_offset,
        });
    }

    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let draw_call = self.current_draw_call();
        draw_call.viewport = glam::Vec4::new(x, y, width, height);
    }

    pub fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) {
        let draw_call = self.current_draw_call();
        draw_call.scissor = [x, y, width, height];
    }

    pub fn set_blend_state(&mut self, blend_state: Option<wgpu::BlendState>) {
        let draw_call = self.current_draw_call();
        draw_call.blend_state = blend_state;
    }

    pub fn set_cull_mode(&mut self, cull_mode: Option<wgpu::Face>) {
        let draw_call = self.current_draw_call();
        draw_call.cull_mode = cull_mode;
    }

    pub fn set_uniforms(
        &mut self,
        fog_color: glam::Vec4,
        blend_color: glam::Vec4,
        prim_color: glam::Vec4,
        env_color: glam::Vec4,
        key_center: glam::Vec3,
        key_scale: glam::Vec3,
        prim_lod: glam::Vec2,
        convert_k: [i32; 6],
    ) {
        let draw_call = self.current_draw_call();
        draw_call.uniforms = GraphicsIntermediateUniforms {
            fog_color,
            blend_color,
            prim_color,
            env_color,
            key_center,
            key_scale,
            prim_lod,
            convert_k,
        };
    }

    pub fn set_vbo(&mut self, vbo: Vec<u8>, num_tris: usize) {
        let draw_call = self.current_draw_call();
        draw_call.vbo = GraphicsIntermediateVBO { vbo, num_tris };
        draw_call.finalize();

        // start a new draw call that's a copy of the current one
        // we do this cause atm we only set properties on changes
        self.new_draw_call();
    }
}
