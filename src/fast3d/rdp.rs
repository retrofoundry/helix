use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use imgui_glow_renderer::glow;
use log::trace;
use wgpu::{BlendState, CompareFunction};

use crate::fast3d::gbi::utils::{other_mode_l_uses_alpha, other_mode_l_uses_texture_edge};

use super::{
    gbi::{
        defines::Viewport,
        utils::{
            get_cycle_type_from_other_mode_h, get_textfilter_from_other_mode_h, translate_cull_mode,
        },
    },
    graphics::GraphicsContext,
    rsp::RSPGeometry,
    utils::{
        color::Color,
        color_combiner::CombineParams,
        texture::{
            translate_tile_ci4, translate_tile_ci8, translate_tile_i4, translate_tile_i8,
            translate_tile_ia16, translate_tile_ia4, translate_tile_ia8, translate_tile_rgba16,
            translate_tile_rgba32, translate_tlut, ImageFormat, ImageSize, TextFilt, Texture,
            TextureImageState, TextureLUT, TextureManager, TextureState,
        },
        tile::TileDescriptor,
    },
};

use crate::fast3d::graphics::opengl_program::OpenGLProgram;
use farbe::image::n64::ImageSize as FarbeImageSize;

pub const SCREEN_WIDTH: f32 = 320.0;
pub const SCREEN_HEIGHT: f32 = 240.0;
const MAX_VBO_SIZE: usize = 256;
const TEXTURE_CACHE_MAX_SIZE: usize = 500;
const MAX_TEXTURE_SIZE: usize = 4096;
pub const NUM_TILE_DESCRIPTORS: usize = 8;
pub const MAX_BUFFERED: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const ZERO: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OutputDimensions {
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: f32,
}

impl OutputDimensions {
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
        aspect_ratio: 0.0,
    };
}

pub struct RenderingState {
    pub depth_compare: CompareFunction,
    pub depth_test: bool,
    pub depth_write: bool,
    pub polygon_offset: bool,
    pub blend_enabled: bool,
    pub blend_state: BlendState,
    pub viewport: Rect,
    pub scissor: Rect,
    pub shader_program_hash: u64,
    pub textures: [Texture; 2],
    pub cull_mode: Option<wgpu::Face>,
    pub blend_color: Color,
}

impl RenderingState {
    pub const EMPTY: Self = Self {
        depth_compare: CompareFunction::Always,
        depth_test: false,
        depth_write: false,
        polygon_offset: false,
        blend_enabled: false,
        blend_state: BlendState::REPLACE,
        viewport: Rect::ZERO,
        scissor: Rect::ZERO,
        shader_program_hash: 0,
        textures: [Texture::EMPTY; 2],
        cull_mode: None,
        blend_color: Color::TRANSPARENT,
    };
}

pub enum OtherModeLayoutL {
    // non-render-mode fields
    G_MDSFT_ALPHACOMPARE = 0,
    G_MDSFT_ZSRCSEL = 2,
    // cycle-independent render-mode bits
    AA_EN = 3,
    Z_CMP = 4,
    Z_UPD = 5,
    IM_RD = 6,
    CLR_ON_CVG = 7,
    CVG_DST = 8,
    ZMODE = 10,
    CVG_X_ALPHA = 12,
    ALPHA_CVG_SEL = 13,
    FORCE_BL = 14,
    // bit 15 unused, was "TEX_EDGE"
    // cycle-dependent render-mode bits
    B_2 = 16,
    B_1 = 18,
    M_2 = 20,
    M_1 = 22,
    A_2 = 24,
    A_1 = 26,
    P_2 = 28,
    P_1 = 30,
}

pub enum OtherModeH_Layout {
    G_MDSFT_BLENDMASK = 0,
    G_MDSFT_ALPHADITHER = 4,
    G_MDSFT_RGBDITHER = 6,
    G_MDSFT_COMBKEY = 8,
    G_MDSFT_TEXTCONV = 9,
    G_MDSFT_TEXTFILT = 12,
    G_MDSFT_TEXTLUT = 14,
    G_MDSFT_TEXTLOD = 16,
    G_MDSFT_TEXTDETAIL = 17,
    G_MDSFT_TEXTPERSP = 19,
    G_MDSFT_CYCLETYPE = 20,
    G_MDSFT_COLORDITHER = 22,
    G_MDSFT_PIPELINE = 23,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtherModeHCycleType {
    G_CYC_1CYCLE = 0,
    G_CYC_2CYCLE = 1,
    G_CYC_COPY = 2,
    G_CYC_FILL = 3,
}

enum ZMode {
    ZMODE_OPA = 0,
    ZMODE_INTER = 1,
    ZMODE_XLU = 2, // translucent
    ZMODE_DEC = 3,
}

pub enum BlendParamPMColor {
    G_BL_CLR_IN = 0,
    G_BL_CLR_MEM = 1,
    G_BL_CLR_BL = 2,
    G_BL_CLR_FOG = 3,
}

enum BlendParamA {
    G_BL_A_IN = 0,
    G_BL_A_FOG = 1,
    G_BL_A_SHADE = 2,
    G_BL_0 = 3,
}

pub enum BlendParamB {
    G_BL_1MA = 0,
    G_BL_A_MEM = 1,
    G_BL_1 = 2,
    G_BL_0 = 3,
}

pub enum AlphaCompare {
    G_AC_NONE = 0,
    G_AC_THRESHOLD = 1,
    G_AC_DITHER = 3,
}

pub struct TMEMMapEntry {
    pub address: usize,
}

impl TMEMMapEntry {
    pub fn new(address: usize) -> Self {
        Self { address }
    }
}

pub struct RDP {
    pub output_dimensions: OutputDimensions,
    pub rendering_state: RenderingState,

    pub texture_manager: TextureManager,

    pub texture_state: TextureState,
    pub texture_image_state: TextureImageState, // coming via GBI (texture to load)
    pub tile_descriptors: [TileDescriptor; NUM_TILE_DESCRIPTORS],
    pub tmem_map: HashMap<u16, TMEMMapEntry>, // tmem address -> texture image state address
    pub textures_changed: [bool; 2],

    pub shader_cache: HashMap<u64, OpenGLProgram>,

    pub viewport: Rect,
    pub scissor: Rect,
    pub viewport_or_scissor_changed: bool,

    pub combine: CombineParams,
    pub other_mode_l: u32,
    pub other_mode_h: u32,

    pub buf_vbo: [f32; MAX_VBO_SIZE * (26 * 3)], // 3 vertices in a triangle and 26 floats per vtx
    pub buf_vbo_len: usize,
    pub buf_vbo_num_tris: usize,

    pub env_color: Color,
    pub fog_color: Color,
    pub prim_color: Color,
    pub blend_color: Color,
    pub fill_color: Color,

    pub depth_image: usize,
    pub color_image: usize,
}

impl RDP {
    pub fn new() -> Self {
        RDP {
            output_dimensions: OutputDimensions::ZERO,
            rendering_state: RenderingState::EMPTY,

            texture_manager: TextureManager::new(TEXTURE_CACHE_MAX_SIZE),

            texture_state: TextureState::EMPTY,
            texture_image_state: TextureImageState::EMPTY,
            tile_descriptors: [TileDescriptor::EMPTY; 8],
            tmem_map: HashMap::new(),
            textures_changed: [false; 2],

            shader_cache: HashMap::new(),

            viewport: Rect::ZERO,
            scissor: Rect::ZERO,
            viewport_or_scissor_changed: false,

            combine: CombineParams::ZERO,
            other_mode_l: 0,
            other_mode_h: 0,

            buf_vbo: [0.0; MAX_VBO_SIZE * (26 * 3)],
            buf_vbo_len: 0,
            buf_vbo_num_tris: 0,

            env_color: Color::TRANSPARENT,
            fog_color: Color::TRANSPARENT,
            prim_color: Color::TRANSPARENT,
            blend_color: Color::TRANSPARENT,
            fill_color: Color::TRANSPARENT,

            depth_image: 0,
            color_image: 0,
        }
    }

    pub fn reset(&mut self) {}

    // Viewport

    pub fn calculate_and_set_viewport(&mut self, viewport: Viewport) {
        let mut width = 2.0 * viewport.vscale[0] as f32 / 4.0;
        let mut height = 2.0 * viewport.vscale[1] as f32 / 4.0;
        let mut x = viewport.vtrans[0] as f32 / 4.0 - width / 2.0;
        let mut y = SCREEN_HEIGHT - ((viewport.vtrans[1] as f32 / 4.0) + height / 2.0);

        width *= self.scaled_x();
        height *= self.scaled_y();
        x *= self.scaled_x();
        y *= self.scaled_y();

        self.viewport.x = x as u16;
        self.viewport.y = y as u16;
        self.viewport.width = width as u16;
        self.viewport.height = height as u16;

        self.viewport_or_scissor_changed = true;
    }

    pub fn adjust_x_for_viewport(&self, x: f32) -> f32 {
        x * (4.0 / 3.0)
            / (self.output_dimensions.width as f32 / self.output_dimensions.height as f32)
    }

    // Textures

    pub fn lookup_texture(
        &mut self,
        gl_context: &glow::Context,
        gfx_context: &GraphicsContext,
        tmem_index: usize,
        fmt: ImageFormat,
        siz: ImageSize,
    ) -> bool {
        if let Some(value) = self.texture_manager.lookup(
            gl_context,
            gfx_context,
            tmem_index,
            self.texture_image_state.address,
            fmt,
            siz,
        ) {
            self.rendering_state.textures[tmem_index] = *value;
            true
        } else {
            let value = self.texture_manager.insert(
                gl_context,
                gfx_context,
                tmem_index,
                self.texture_image_state.address,
                fmt,
                siz,
            );
            self.rendering_state.textures[tmem_index] = *value;
            false
        }
    }

    pub fn import_tile_texture(
        &mut self,
        gl_context: &glow::Context,
        gfx_context: &GraphicsContext,
        tmem_index: usize,
    ) {
        let tile = self.tile_descriptors[self.texture_state.tile as usize];
        let format = tile.format as u32;
        let size = tile.size as u32;
        let width = tile.get_width() as u32;
        let height = tile.get_height() as u32;

        if self.lookup_texture(gl_context, gfx_context, tmem_index, tile.format, tile.size) {
            return;
        }

        let tmap_entry = self.tmem_map.get(&tile.tmem).unwrap();
        let texture_address = tmap_entry.address;

        // TODO: figure out how to find the size of bytes in the texture
        let texture_data = unsafe {
            std::slice::from_raw_parts(texture_address as *const u8, MAX_TEXTURE_SIZE * 4)
        };

        let texture = match (format << 4) | size {
            x if x
                == ((ImageFormat::G_IM_FMT_RGBA as u32) << 4 | ImageSize::G_IM_SIZ_16b as u32) =>
            {
                translate_tile_rgba16(texture_data, width, height)
            }
            x if x
                == ((ImageFormat::G_IM_FMT_RGBA as u32) << 4 | ImageSize::G_IM_SIZ_32b as u32) =>
            {
                translate_tile_rgba32(texture_data, width, height)
            }
            x if x == ((ImageFormat::G_IM_FMT_IA as u32) << 4 | ImageSize::G_IM_SIZ_4b as u32) => {
                translate_tile_ia4(texture_data, width, height)
            }
            x if x == ((ImageFormat::G_IM_FMT_IA as u32) << 4 | ImageSize::G_IM_SIZ_8b as u32) => {
                translate_tile_ia8(texture_data, width, height)
            }
            x if x == ((ImageFormat::G_IM_FMT_IA as u32) << 4 | ImageSize::G_IM_SIZ_16b as u32) => {
                translate_tile_ia16(texture_data, width, height)
            }
            x if x == ((ImageFormat::G_IM_FMT_I as u32) << 4 | ImageSize::G_IM_SIZ_4b as u32) => {
                translate_tile_i4(texture_data, width, height)
            }
            x if x == ((ImageFormat::G_IM_FMT_I as u32) << 4 | ImageSize::G_IM_SIZ_8b as u32) => {
                translate_tile_i8(texture_data, width, height)
            }
            x if x == ((ImageFormat::G_IM_FMT_CI as u32) << 4 | ImageSize::G_IM_SIZ_4b as u32) => {
                let pal_addr = self
                    .tmem_map
                    .get(&(u16::MAX - tmem_index as u16))
                    .unwrap()
                    .address;
                let texlut: TextureLUT = TextureLUT::from_u32((self.other_mode_h >> 14) & 0x3);
                let palette = translate_tlut(pal_addr, FarbeImageSize::S4B, &texlut);
                translate_tile_ci4(texture_data, &palette, width, height)
            }
            x if x == ((ImageFormat::G_IM_FMT_CI as u32) << 4 | ImageSize::G_IM_SIZ_8b as u32) => {
                let pal_addr = self
                    .tmem_map
                    .get(&(u16::MAX - tmem_index as u16))
                    .unwrap()
                    .address;
                let texlut: TextureLUT = TextureLUT::from_u32((self.other_mode_h >> 14) & 0x3);
                let palette = translate_tlut(pal_addr, FarbeImageSize::S8B, &texlut);
                translate_tile_ci8(texture_data, &palette, width, height)
            }
            _ => {
                // TODO: Create an empty texture?
                panic!("Unsupported texture format: {:?} {:?}", format, size);
            }
        };

        let texture = texture.as_ptr() as *const u8;
        gfx_context
            .api
            .upload_texture(gl_context, texture, width as i32, height as i32);
    }

    pub fn uses_texture1(&self) -> bool {
        get_cycle_type_from_other_mode_h(self.other_mode_h) == OtherModeHCycleType::G_CYC_2CYCLE
            && self.combine.uses_texture1()
    }

    pub fn flush_textures(&mut self, gl_context: &glow::Context, gfx_context: &GraphicsContext) {
        // if textures are not on, then we have no textures to flush
        // if !self.texture_state.on {
        //     return;
        // }

        let lod_en = (self.other_mode_h >> 16 & 0x1) != 0;
        if lod_en {
            // TODO: Support mip-mapping
            trace!("Mip-mapping is enabled, but not supported yet");
            assert!(false);
        } else {
            // we're in TILE mode. Let's check if we're in two-cycle mode.
            // let cycle_type = RDP::get_cycle_type_from_other_mode_h(self.other_mode_h);
            // assert!(
            //     cycle_type == OtherModeHCycleType::G_CYC_1CYCLE
            //         || cycle_type == OtherModeHCycleType::G_CYC_2CYCLE
            // );

            for i in 0..2 {
                if i == 0 || self.uses_texture1() {
                    if self.textures_changed[i as usize] {
                        self.flush(gl_context, gfx_context);
                        self.import_tile_texture(gl_context, gfx_context, i as usize);
                        self.textures_changed[i as usize] = false;
                    }

                    let tile_descriptor =
                        self.tile_descriptors[(self.texture_state.tile + i) as usize];
                    let linear_filter =
                        get_textfilter_from_other_mode_h(self.other_mode_h) != TextFilt::G_TF_POINT;
                    let texture = self.rendering_state.textures[i as usize];
                    if linear_filter != texture.linear_filter
                        || tile_descriptor.cm_s != texture.cms
                        || tile_descriptor.cm_t != texture.cmt
                    {
                        gfx_context.api.set_sampler_parameters(
                            gl_context,
                            i as i32,
                            linear_filter,
                            tile_descriptor.cm_s as u32,
                            tile_descriptor.cm_t as u32,
                        );
                        self.rendering_state.textures[i as usize].linear_filter = linear_filter;
                        self.rendering_state.textures[i as usize].cms = tile_descriptor.cm_s;
                        self.rendering_state.textures[i as usize].cmt = tile_descriptor.cm_t;
                    }
                }
            }
        }
    }

    pub fn flush(&mut self, gl_context: &glow::Context, gfx_context: &GraphicsContext) {
        if self.buf_vbo_len > 0 {
            gfx_context.api.draw_triangles(
                gl_context,
                &self.buf_vbo as *const f32,
                self.buf_vbo_len,
                self.buf_vbo_num_tris,
            );
            self.buf_vbo_len = 0;
            self.buf_vbo_num_tris = 0;
        }
    }

    // MARK: - Shader Programs

    pub fn shader_program_hash(&mut self) -> u64 {
        let mut hasher = DefaultHasher::new();

        self.other_mode_h.hash(&mut hasher);
        self.other_mode_l.hash(&mut hasher);
        self.combine.hash(&mut hasher);
        self.tile_descriptors.hash(&mut hasher);

        hasher.finish()
    }

    pub fn lookup_or_create_program(
        &mut self,
        gl_context: &glow::Context,
        gfx_context: &GraphicsContext,
    ) -> u64 {
        let hash = self.shader_program_hash();
        if let Some(_program) = self.shader_cache.get(&hash) {
            return hash;
        }

        let mut program = OpenGLProgram::new(self.other_mode_h, self.other_mode_l, self.combine, self.tile_descriptors);
        program.init();
        program.preprocess();

        gfx_context.api.compile_program(gl_context, &mut program);

        self.shader_cache.insert(hash, program);

        hash
    }

    // MARK: - Blend

    fn translate_blend_mode(
        &mut self,
        gl_context: &glow::Context,
        gfx_context: &GraphicsContext,
        render_mode: u32,
    ) {
        let zmode: u32 = self.other_mode_l >> (OtherModeLayoutL::ZMODE as u32) & 0x03;

        // handle depth compare
        if self.other_mode_l & (1 << OtherModeLayoutL::Z_CMP as u32) != 0 {
            let depth_compare = match zmode {
                x if x == ZMode::ZMODE_OPA as u32 => CompareFunction::Less,
                x if x == ZMode::ZMODE_INTER as u32 => CompareFunction::Less, // TODO: Understand this
                x if x == ZMode::ZMODE_XLU as u32 => CompareFunction::Less,
                x if x == ZMode::ZMODE_DEC as u32 => CompareFunction::LessEqual,
                _ => panic!("Unknown ZMode"),
            };

            if depth_compare != self.rendering_state.depth_compare {
                self.flush(gl_context, gfx_context);
                gfx_context.api.set_depth_compare(gl_context, depth_compare);
                self.rendering_state.depth_compare = depth_compare;
            }
        } else if self.rendering_state.depth_compare != CompareFunction::Always {
            self.flush(gl_context, gfx_context);
            gfx_context
                .api
                .set_depth_compare(gl_context, CompareFunction::Always);
            self.rendering_state.depth_compare = CompareFunction::Always;
        }

        // handle depth write
        let depth_write = render_mode & (1 << OtherModeLayoutL::Z_UPD as u32) != 0;
        if depth_write != self.rendering_state.depth_write {
            self.flush(gl_context, gfx_context);
            gfx_context.api.set_depth_write(gl_context, depth_write);
            self.rendering_state.depth_write = depth_write;
        }

        // handle polygon offset (slope scale depth bias)
        let polygon_offset = zmode == ZMode::ZMODE_DEC as u32;
        if polygon_offset != self.rendering_state.polygon_offset {
            self.flush(gl_context, gfx_context);
            gfx_context
                .api
                .set_polygon_offset(gl_context, polygon_offset);
            self.rendering_state.polygon_offset = polygon_offset;
        }

        // handle alpha blending
        let do_blend = other_mode_l_uses_alpha(self.other_mode_l)
            || other_mode_l_uses_texture_edge(self.other_mode_l);

        if do_blend != self.rendering_state.blend_enabled {
            let blend_state = BlendState::ALPHA_BLENDING;

            self.flush(gl_context, gfx_context);
            gfx_context
                .api
                .set_blend_state(gl_context, do_blend, blend_state, self.blend_color);
            self.rendering_state.blend_enabled = do_blend;
        }
    }

    pub fn update_render_state(
        &mut self,
        gl_context: &glow::Context,
        gfx_context: &mut GraphicsContext,
        geometry_mode: u32,
    ) {
        let depth_test = geometry_mode & RSPGeometry::G_ZBUFFER as u32 != 0;
        if depth_test != self.rendering_state.depth_test {
            self.flush(gl_context, gfx_context);
            gfx_context.api.set_depth_test(gl_context, depth_test);
            self.rendering_state.depth_test = depth_test;
        }

        let cull_mode = translate_cull_mode(geometry_mode);
        if cull_mode != self.rendering_state.cull_mode {
            self.flush(gl_context, gfx_context);
            gfx_context.api.set_cull_mode(gl_context, cull_mode);
            self.rendering_state.cull_mode = cull_mode;
        }

        self.translate_blend_mode(gl_context, gfx_context, self.other_mode_l);

        if self.viewport_or_scissor_changed {
            let viewport = self.viewport;
            if viewport != self.rendering_state.viewport {
                self.flush(gl_context, gfx_context);
                gfx_context.api.set_viewport(
                    gl_context,
                    viewport.x as i32,
                    viewport.y as i32,
                    viewport.width as i32,
                    viewport.height as i32,
                );
                self.rendering_state.viewport = viewport;
            }
            let scissor = self.scissor;
            if scissor != self.rendering_state.scissor {
                self.flush(gl_context, gfx_context);
                gfx_context.api.set_scissor(
                    gl_context,
                    scissor.x as i32,
                    scissor.y as i32,
                    scissor.width as i32,
                    scissor.height as i32,
                );
                self.rendering_state.scissor = scissor;
            }
            self.viewport_or_scissor_changed = false;
        }
    }

    // MARK: - Helpers

    pub fn scaled_x(&self) -> f32 {
        self.output_dimensions.width as f32 / SCREEN_WIDTH
    }

    pub fn scaled_y(&self) -> f32 {
        self.output_dimensions.height as f32 / SCREEN_HEIGHT
    }

    pub fn add_to_buf_vbo(&mut self, data: f32) {
        self.buf_vbo[self.buf_vbo_len] = data;
        self.buf_vbo_len += 1;
    }
}
