use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use glam::{Vec2, Vec3, Vec4};
use log::trace;
use wgpu::{BlendState, CompareFunction};

use crate::fast3d::gbi::utils::{other_mode_l_uses_alpha, other_mode_l_uses_texture_edge};

use super::graphics::GraphicsIntermediateDevice;
use super::utils::texture::Texture;
use super::{
    gbi::{
        defines::Viewport,
        utils::{
            get_cycle_type_from_other_mode_h, get_textfilter_from_other_mode_h, translate_cull_mode,
        },
    },
    rsp::RSPGeometry,
    utils::{
        color::Color,
        color_combiner::CombineParams,
        texture::{
            translate_tile_ci4, translate_tile_ci8, translate_tile_i4, translate_tile_i8,
            translate_tile_ia16, translate_tile_ia4, translate_tile_ia8, translate_tile_rgba16,
            translate_tile_rgba32, translate_tlut, ImageFormat, ImageSize, TextFilt,
            TextureImageState, TextureLUT, TextureState,
        },
        tile::TileDescriptor,
    },
};

use farbe::image::n64::ImageSize as FarbeImageSize;

pub const SCREEN_WIDTH: f32 = 320.0;
pub const SCREEN_HEIGHT: f32 = 240.0;
const MAX_VBO_SIZE: usize = 256;
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

    pub texture_state: TextureState,
    pub texture_image_state: TextureImageState, // coming via GBI (texture to load)
    pub tile_descriptors: [TileDescriptor; NUM_TILE_DESCRIPTORS],
    pub tmem_map: HashMap<u16, TMEMMapEntry>, // tmem address -> texture image state address
    pub textures_changed: [bool; 2],

    pub viewport: Rect,
    pub scissor: Rect,
    pub viewport_or_scissor_changed: bool,

    pub combine: CombineParams,
    pub other_mode_l: u32,
    pub other_mode_h: u32,

    pub buf_vbo: [f32; MAX_VBO_SIZE * (26 * 3)], // 3 vertices in a triangle and 26 floats per vtx
    pub buf_vbo_len: usize,
    pub buf_vbo_num_tris: usize,

    pub env_color: Vec4,
    pub fog_color: Vec4,
    pub prim_color: Vec4,
    pub blend_color: Vec4,
    pub fill_color: Vec4,

    pub prim_lod: Vec2,

    pub convert_k: [i32; 6],
    pub key_center: Vec3,
    pub key_scale: Vec3,

    pub depth_image: usize,
    pub color_image: usize,
}

impl RDP {
    pub fn new() -> Self {
        RDP {
            output_dimensions: OutputDimensions::ZERO,
            rendering_state: RenderingState::EMPTY,

            texture_state: TextureState::EMPTY,
            texture_image_state: TextureImageState::EMPTY,
            tile_descriptors: [TileDescriptor::EMPTY; 8],
            tmem_map: HashMap::new(),
            textures_changed: [false; 2],

            viewport: Rect::ZERO,
            scissor: Rect::ZERO,
            viewport_or_scissor_changed: false,

            combine: CombineParams::ZERO,
            other_mode_l: 0,
            other_mode_h: 0,

            buf_vbo: [0.0; MAX_VBO_SIZE * (26 * 3)],
            buf_vbo_len: 0,
            buf_vbo_num_tris: 0,

            env_color: Vec4::ZERO,
            fog_color: Vec4::ZERO,
            prim_color: Vec4::ZERO,
            blend_color: Vec4::ZERO,
            fill_color: Vec4::ZERO,

            prim_lod: Vec2::ZERO,

            convert_k: [0; 6],
            key_center: Vec3::ZERO,
            key_scale: Vec3::ZERO,

            depth_image: 0,
            color_image: 0,
        }
    }

    pub fn reset(&mut self) {
        self.combine = CombineParams::ZERO;
        self.other_mode_l = 0;
        self.other_mode_h = 0;
        self.env_color = Vec4::ZERO;
        self.fog_color = Vec4::ZERO;
        self.prim_color = Vec4::ZERO;
        self.blend_color = Vec4::ZERO;
        self.fill_color = Vec4::ZERO;
        self.prim_lod = Vec2::ZERO;
        self.key_center = Vec3::ZERO;
        self.key_scale = Vec3::ZERO;
        self.convert_k = [0; 6];
    }

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

    pub fn import_tile_texture(
        &mut self,
        gfx_device: &mut GraphicsIntermediateDevice,
        tmem_index: usize,
    ) {
        let tile = self.tile_descriptors[self.texture_state.tile as usize];
        let format = tile.format as u32;
        let size = tile.size as u32;
        let width = tile.get_width() as u32;
        let height = tile.get_height() as u32;

        let tmap_entry = self.tmem_map.get(&tile.tmem).unwrap();
        let texture_address = tmap_entry.address;

        if let Some(hash) =
            gfx_device
                .texture_cache
                .contains(texture_address, tile.format, tile.size)
        {
            gfx_device.set_texture(tmem_index, hash);
            return;
        }

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

        let hash = gfx_device.texture_cache.insert(
            texture_address,
            tile.format,
            tile.size,
            width,
            height,
            texture,
        );
        gfx_device.set_texture(tmem_index, hash);
    }

    pub fn uses_texture1(&self) -> bool {
        get_cycle_type_from_other_mode_h(self.other_mode_h) == OtherModeHCycleType::G_CYC_2CYCLE
            && self.combine.uses_texture1()
    }

    pub fn flush_textures(&mut self, gfx_device: &mut GraphicsIntermediateDevice) {
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
                        self.flush(gfx_device);
                        gfx_device.clear_textures(i as usize);

                        self.import_tile_texture(gfx_device, i as usize);
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
                        gfx_device.set_sampler_parameters(
                            i as usize,
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

    pub fn flush(&mut self, gfx_device: &mut GraphicsIntermediateDevice) {
        if self.buf_vbo_len > 0 {
            let vbo = unsafe {
                std::slice::from_raw_parts(
                    (&self.buf_vbo as *const f32) as *const u8,
                    self.buf_vbo_len * 4,
                )
            };
            gfx_device.set_vbo(vbo.to_vec(), self.buf_vbo_num_tris);
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

    // MARK: - Blend

    fn process_depth_params(
        &mut self,
        gfx_device: &mut GraphicsIntermediateDevice,
        geometry_mode: u32,
        render_mode: u32,
    ) {
        let depth_test = geometry_mode & RSPGeometry::G_ZBUFFER as u32 != 0;
        if depth_test != self.rendering_state.depth_test {
            self.flush(gfx_device);
            self.rendering_state.depth_test = depth_test;
        }

        let zmode: u32 = self.other_mode_l >> (OtherModeLayoutL::ZMODE as u32) & 0x03;

        // handle depth compare
        let depth_compare = if self.other_mode_l & (1 << OtherModeLayoutL::Z_CMP as u32) != 0 {
            let depth_compare = match zmode {
                x if x == ZMode::ZMODE_OPA as u32 => CompareFunction::Less,
                x if x == ZMode::ZMODE_INTER as u32 => CompareFunction::Less, // TODO: Understand this
                x if x == ZMode::ZMODE_XLU as u32 => CompareFunction::Less,
                x if x == ZMode::ZMODE_DEC as u32 => CompareFunction::LessEqual,
                _ => panic!("Unknown ZMode"),
            };

            if depth_compare != self.rendering_state.depth_compare {
                self.flush(gfx_device);
                self.rendering_state.depth_compare = depth_compare;
            }

            depth_compare
        } else {
            if self.rendering_state.depth_compare != CompareFunction::Always {
                self.flush(gfx_device);
                self.rendering_state.depth_compare = CompareFunction::Always;
            }

            CompareFunction::Always
        };

        // handle depth write
        let depth_write = render_mode & (1 << OtherModeLayoutL::Z_UPD as u32) != 0;
        if depth_write != self.rendering_state.depth_write {
            self.flush(gfx_device);
            self.rendering_state.depth_write = depth_write;
        }

        // handle polygon offset (slope scale depth bias)
        let polygon_offset = zmode == ZMode::ZMODE_DEC as u32;
        if polygon_offset != self.rendering_state.polygon_offset {
            self.flush(gfx_device);
            self.rendering_state.polygon_offset = polygon_offset;
        }

        gfx_device.set_depth_stencil_params(depth_test, depth_write, depth_compare, polygon_offset);
    }

    pub fn update_render_state(
        &mut self,
        gfx_device: &mut GraphicsIntermediateDevice,
        geometry_mode: u32,
    ) {
        let cull_mode = translate_cull_mode(geometry_mode);
        if cull_mode != self.rendering_state.cull_mode {
            self.flush(gfx_device);
            gfx_device.set_cull_mode(cull_mode);
            self.rendering_state.cull_mode = cull_mode;
        }

        self.process_depth_params(gfx_device, geometry_mode, self.other_mode_l);

        // handle alpha blending
        let do_blend = other_mode_l_uses_alpha(self.other_mode_l)
            || other_mode_l_uses_texture_edge(self.other_mode_l);

        if do_blend != self.rendering_state.blend_enabled {
            let blend_state = if do_blend {
                Some(BlendState::ALPHA_BLENDING)
            } else {
                None
            };

            self.flush(gfx_device);
            gfx_device.set_blend_state(blend_state);
            self.rendering_state.blend_enabled = do_blend;
        }

        // handle viewport and scissor
        if self.viewport_or_scissor_changed {
            let viewport = self.viewport;
            if viewport != self.rendering_state.viewport {
                self.flush(gfx_device);
                gfx_device.set_viewport(
                    viewport.x as f32,
                    viewport.y as f32,
                    viewport.width as f32,
                    viewport.height as f32,
                );
                self.rendering_state.viewport = viewport;
            }
            let scissor = self.scissor;
            if scissor != self.rendering_state.scissor {
                self.flush(gfx_device);
                gfx_device.set_scissor(
                    scissor.x as u32,
                    scissor.y as u32,
                    scissor.width as u32,
                    scissor.height as u32,
                );
                self.rendering_state.scissor = scissor;
            }
            self.viewport_or_scissor_changed = false;
        }
    }

    // MARK: - Setters

    pub fn set_convert(&mut self, k0: i32, k1: i32, k2: i32, k3: i32, k4: i32, k5: i32) {
        self.convert_k[0] = k0;
        self.convert_k[1] = k1;
        self.convert_k[2] = k2;
        self.convert_k[3] = k3;
        self.convert_k[4] = k4;
        self.convert_k[5] = k5;
    }

    pub fn set_key_r(&mut self, cr: u32, sr: u32, _wr: u32) {
        // TODO: Figure out how to use width
        self.key_center.x = cr as f32 / 255.0;
        self.key_scale.x = sr as f32 / 255.0;
    }

    pub fn set_key_gb(&mut self, cg: u32, sg: u32, _wg: u32, cb: u32, sb: u32, _wb: u32) {
        // TODO: Figure out how to use width
        self.key_center.y = cg as f32 / 255.0;
        self.key_center.z = cb as f32 / 255.0;
        self.key_scale.y = sg as f32 / 255.0;
        self.key_scale.z = sb as f32 / 255.0;
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
