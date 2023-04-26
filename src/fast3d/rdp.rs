use std::collections::HashMap;

use log::trace;
use wgpu::{BlendComponent, BlendFactor, BlendOperation, BlendState, CompareFunction, Face};

use super::{
    gbi::defines::Viewport,
    graphics::{CullMode, GraphicsContext, ShaderProgram},
    rcp::RCP,
    rsp::{RSPGeometry, StagingVertex},
    utils::{
        color_combiner::{
            ColorCombiner, ColorCombinerManager, CombineParams, ACMUX, CCMUX, SHADER,
        },
        texture::{
            translate_tile_ci4, translate_tile_ci8, translate_tile_i4, translate_tile_i8,
            translate_tile_ia16, translate_tile_ia4, translate_tile_ia8, translate_tile_rgba16,
            translate_tile_rgba32, translate_tlut, ImageFormat, ImageSize, TextFilt, Texture,
            TextureImageState, TextureLUT, TextureManager, TextureState,
        },
        tile::TileDescriptor,
    },
};

use farbe::image::n64::ImageSize as FarbeImageSize;

pub const SCREEN_WIDTH: f32 = 320.0;
pub const SCREEN_HEIGHT: f32 = 240.0;
const MAX_VBO_SIZE: usize = 256;
const TEXTURE_CACHE_MAX_SIZE: usize = 500;
const MAX_TEXTURE_SIZE: usize = 4096;
const NUM_TILE_DESCRIPTORS: usize = 8;
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
    pub blend_state: BlendState,
    pub viewport: Rect,
    pub scissor: Rect,
    pub shader_program: *mut ShaderProgram,
    pub textures: [Texture; 2],
    pub cull_mode: CullMode,
}

impl RenderingState {
    pub const EMPTY: Self = Self {
        depth_compare: CompareFunction::Always,
        depth_test: false,
        depth_write: false,
        polygon_offset: false,
        blend_state: BlendState::REPLACE,
        viewport: Rect::ZERO,
        scissor: Rect::ZERO,
        shader_program: std::ptr::null_mut(),
        textures: [Texture::EMPTY; 2],
        cull_mode: CullMode::None,
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

    pub color_combiner_manager: ColorCombinerManager,

    pub viewport: Rect,
    pub scissor: Rect,
    pub viewport_or_scissor_changed: bool,

    pub combine: CombineParams,
    pub other_mode_l: u32,
    pub other_mode_h: u32,

    pub buf_vbo: [f32; MAX_VBO_SIZE * (26 * 3)], // 3 vertices in a triangle and 26 floats per vtx
    pub buf_vbo_len: usize,
    pub buf_vbo_num_tris: usize,

    pub env_color: [u8; 4],
    pub fog_color: [u8; 4],
    pub prim_color: [u8; 4],
    pub fill_color: [u8; 4],

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

            color_combiner_manager: ColorCombinerManager::new(),

            viewport: Rect::ZERO,
            scissor: Rect::ZERO,
            viewport_or_scissor_changed: false,

            combine: CombineParams::ZERO,
            other_mode_l: 0,
            other_mode_h: 0,

            buf_vbo: [0.0; MAX_VBO_SIZE * (26 * 3)],
            buf_vbo_len: 0,
            buf_vbo_num_tris: 0,

            env_color: [0; 4],
            fog_color: [0; 4],
            prim_color: [0; 4],
            fill_color: [0; 4],

            depth_image: 0,
            color_image: 0,
        }
    }

    pub fn reset(&mut self) {}

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

    pub fn lookup_texture(
        &mut self,
        gfx_context: &GraphicsContext,
        tmem_index: usize,
        fmt: ImageFormat,
        siz: ImageSize,
    ) -> bool {
        if let Some(value) = self.texture_manager.lookup(
            gfx_context,
            tmem_index,
            self.texture_image_state.address as usize,
            fmt,
            siz,
        ) {
            self.rendering_state.textures[tmem_index as usize] = *value;
            true
        } else {
            let value = self.texture_manager.insert(
                gfx_context,
                tmem_index,
                self.texture_image_state.address as usize,
                fmt,
                siz,
            );
            self.rendering_state.textures[tmem_index as usize] = *value;
            false
        }
    }

    pub fn import_tile_texture(&mut self, gfx_context: &GraphicsContext, tmem_index: usize) {
        let tile = self.tile_descriptors[self.texture_state.tile as usize];
        let format = tile.format as u32;
        let size = tile.size as u32;
        let width = tile.get_width() as u32;
        let height = tile.get_height() as u32;

        if self.lookup_texture(gfx_context, tmem_index, tile.format, tile.size) {
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
            .upload_texture(texture, width as i32, height as i32);
    }

    pub fn get_cycle_type_from_other_mode_h(mode_h: u32) -> OtherModeHCycleType {
        match (mode_h >> OtherModeH_Layout::G_MDSFT_CYCLETYPE as u32) & 0x03 {
            x if x == OtherModeHCycleType::G_CYC_1CYCLE as u32 => OtherModeHCycleType::G_CYC_1CYCLE,
            x if x == OtherModeHCycleType::G_CYC_2CYCLE as u32 => OtherModeHCycleType::G_CYC_2CYCLE,
            x if x == OtherModeHCycleType::G_CYC_COPY as u32 => OtherModeHCycleType::G_CYC_COPY,
            x if x == OtherModeHCycleType::G_CYC_FILL as u32 => OtherModeHCycleType::G_CYC_FILL,
            _ => panic!("Invalid cycle type"),
        }
    }

    pub fn get_textfilter_from_other_mode_h(mode_h: u32) -> TextFilt {
        match (mode_h >> OtherModeH_Layout::G_MDSFT_TEXTFILT as u32) & 0x3 {
            x if x == TextFilt::G_TF_POINT as u32 => TextFilt::G_TF_POINT,
            x if x == TextFilt::G_TF_AVERAGE as u32 => TextFilt::G_TF_AVERAGE,
            x if x == TextFilt::G_TF_BILERP as u32 => TextFilt::G_TF_BILERP,
            _ => panic!("Invalid text filter"),
        }
    }

    pub fn uses_texture1(&self) -> bool {
        RDP::get_cycle_type_from_other_mode_h(self.other_mode_h)
            == OtherModeHCycleType::G_CYC_2CYCLE
            && self.combine.uses_texture1()
    }

    pub fn flush_textures(&mut self, gfx_context: &GraphicsContext) {
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
                        self.flush(gfx_context);
                        self.import_tile_texture(gfx_context, i as usize);
                        self.textures_changed[i as usize] = false;
                    }

                    let tile_descriptor =
                        self.tile_descriptors[(self.texture_state.tile + i) as usize];
                    let linear_filter = RDP::get_textfilter_from_other_mode_h(self.other_mode_h)
                        != TextFilt::G_TF_POINT;
                    let texture = self.rendering_state.textures[i as usize];
                    if linear_filter != texture.linear_filter
                        || tile_descriptor.cm_s != texture.cms
                        || tile_descriptor.cm_t != texture.cmt
                    {
                        gfx_context.api.set_sampler_parameters(
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

    pub fn adjust_x_for_viewport(&self, x: f32) -> f32 {
        x * (4.0 / 3.0)
            / (self.output_dimensions.width as f32 / self.output_dimensions.height as f32)
    }

    pub fn flush(&mut self, gfx_context: &GraphicsContext) {
        if self.buf_vbo_len > 0 {
            gfx_context.api.draw_triangles(
                &self.buf_vbo as *const f32,
                self.buf_vbo_len,
                self.buf_vbo_num_tris,
            );
            self.buf_vbo_len = 0;
            self.buf_vbo_num_tris = 0;
        }
    }

    pub fn lookup_or_create_shader_program(
        &mut self,
        gfx_context: &GraphicsContext,
        shader_id: u32,
    ) -> *mut ShaderProgram {
        let mut shader_program = gfx_context.api.lookup_shader(shader_id);
        if shader_program.is_null() {
            gfx_context
                .api
                .unload_shader(self.rendering_state.shader_program);
            shader_program = gfx_context.api.create_and_load_new_shader(shader_id);
            self.rendering_state.shader_program = shader_program;
        }

        shader_program
    }

    pub fn create_color_combiner(
        &mut self,
        gfx_context: &GraphicsContext,
        cc_id: u32,
    ) -> &ColorCombiner {
        self.flush(gfx_context);
        self.generate_color_combiner(gfx_context, cc_id);

        let combiner = self.color_combiner_manager.combiners.get(&cc_id).unwrap();
        self.color_combiner_manager.current_combiner = Some(cc_id);

        combiner
    }

    pub fn lookup_or_create_color_combiner(&mut self, gfx_context: &GraphicsContext, cc_id: u32) {
        if let Some(_cc) = self.color_combiner_manager.lookup_color_combiner(cc_id) {
        } else {
            self.create_color_combiner(gfx_context, cc_id);
        }
    }

    pub fn generate_color_combiner(&mut self, gfx_context: &GraphicsContext, cc_id: u32) {
        let mut shader_id = (cc_id >> 24) << 24;
        let mut shader_input_mapping = [[0u8; 4]; 2];

        // parse the color combine pass
        {
            let mut input_number = [0u8; 8];
            let mut next_input_number = SHADER::INPUT_1 as u8;

            for i in 0..4 {
                let mut val = 0;
                match self.combine.c0.get(i) {
                    CCMUX::TEXEL0 => val = SHADER::TEXEL0 as u8,
                    CCMUX::TEXEL1 => val = SHADER::TEXEL1 as u8,
                    CCMUX::TEXEL0_ALPHA => val = SHADER::TEXEL0A as u8,
                    CCMUX::PRIMITIVE | CCMUX::SHADE | CCMUX::ENVIRONMENT | CCMUX::LOD_FRACTION => {
                        let property = self.combine.c0.get(i) as u8;

                        if input_number[property as usize] == 0 {
                            shader_input_mapping[0][(next_input_number - 1) as usize] = property;
                            input_number[property as usize] = next_input_number;
                            next_input_number += 1;
                        }
                        val = input_number[property as usize];
                    }
                    _ => {}
                }

                shader_id |= (val as u32) << (i * 3);
            }
        }

        // parse the alpha combine pass
        {
            let mut input_number = [0u8; 8];
            let mut next_input_number = SHADER::INPUT_1 as u8;

            for i in 0..4 {
                let mut val = 0;
                match self.combine.a0.get(i) {
                    ACMUX::TEXEL0 => val = SHADER::TEXEL0 as u8,
                    ACMUX::TEXEL1 => val = SHADER::TEXEL1 as u8,
                    ACMUX::PRIMITIVE | ACMUX::SHADE | ACMUX::ENVIRONMENT => {
                        let property = self.combine.a0.get(i) as u8;

                        if input_number[property as usize] == 0 {
                            shader_input_mapping[1][(next_input_number - 1) as usize] = property;
                            input_number[property as usize] = next_input_number;
                            next_input_number += 1;
                        }
                        val = input_number[property as usize];
                    }
                    _ => {}
                }

                shader_id |= (val as u32) << (12 + i * 3);
            }
        }

        let shader_program = self.lookup_or_create_shader_program(gfx_context, shader_id);
        let combiner = ColorCombiner::new(shader_id, shader_program, shader_input_mapping);
        self.color_combiner_manager
            .combiners
            .insert(cc_id, combiner);
    }

    fn translate_blend_param_b(param: u32, src: BlendFactor) -> BlendFactor {
        match param {
            x if x == BlendParamB::G_BL_1MA as u32 => {
                if src == BlendFactor::SrcAlpha {
                    BlendFactor::OneMinusSrcAlpha
                } else if src == BlendFactor::One {
                    BlendFactor::Zero
                } else {
                    BlendFactor::One
                }
            }
            x if x == BlendParamB::G_BL_A_MEM as u32 => BlendFactor::DstAlpha,
            x if x == BlendParamB::G_BL_1 as u32 => BlendFactor::One,
            x if x == BlendParamB::G_BL_0 as u32 => BlendFactor::Zero,
            _ => panic!("Unknown Blend Param B: {}", param),
        }
    }

    fn translate_blend_mode(
        &mut self,
        gfx_context: &GraphicsContext,
        render_mode: u32,
    ) -> BlendState {
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
                self.flush(gfx_context);
                gfx_context.api.set_depth_compare(depth_compare as u8);
                self.rendering_state.depth_compare = depth_compare;
            }
        } else {
            if self.rendering_state.depth_compare != CompareFunction::Always {
                self.flush(gfx_context);
                gfx_context
                    .api
                    .set_depth_compare(CompareFunction::Always as u8);
                self.rendering_state.depth_compare = CompareFunction::Always;
            }
        }

        // handle depth write
        let depth_write = render_mode & (1 << OtherModeLayoutL::Z_UPD as u32) != 0;
        if depth_write != self.rendering_state.depth_write {
            self.flush(gfx_context);
            gfx_context.api.set_depth_write(depth_write);
            self.rendering_state.depth_write = depth_write;
        }

        // handle polygon offset (slope scale depth bias)
        let polygon_offset = zmode == ZMode::ZMODE_DEC as u32;
        if polygon_offset != self.rendering_state.polygon_offset {
            self.flush(gfx_context);
            gfx_context.api.set_polygon_offset(polygon_offset);
            self.rendering_state.polygon_offset = polygon_offset;
        }

        let src_color = render_mode >> OtherModeLayoutL::P_2 as u32 & 0x03;
        let src_factor = render_mode >> OtherModeLayoutL::A_2 as u32 & 0x03;
        let dst_color = render_mode >> OtherModeLayoutL::M_2 as u32 & 0x03;
        let dst_factor = render_mode >> OtherModeLayoutL::B_2 as u32 & 0x03;

        let do_blend = render_mode & (1 << OtherModeLayoutL::FORCE_BL as u32) != 0
            && dst_color == BlendParamPMColor::G_BL_CLR_MEM as u32;

        if do_blend {
            assert!(src_color == BlendParamPMColor::G_BL_CLR_IN as u32);

            let blend_src_factor: BlendFactor;
            if src_factor == BlendParamA::G_BL_0 as u32 {
                blend_src_factor = BlendFactor::Zero;
            } else if (render_mode & (1 << OtherModeLayoutL::ALPHA_CVG_SEL as u32)) != 0
                && (render_mode & (1 << OtherModeLayoutL::CVG_X_ALPHA as u32)) == 0
            {
                // this is technically "coverage", admitting blending on edges
                blend_src_factor = BlendFactor::One;
            } else {
                blend_src_factor = BlendFactor::SrcAlpha;
            }

            let blend_component = BlendComponent {
                src_factor: blend_src_factor,
                dst_factor: RDP::translate_blend_param_b(dst_factor, blend_src_factor),
                operation: BlendOperation::Add,
            };

            return BlendState {
                color: blend_component,
                alpha: blend_component,
            };
        } else {
            // without FORCE_BL, blending only happens for AA of internal edges
            // since we are ignoring n64 coverage values and AA, this means "never"
            // if dstColor isn't the framebuffer, we'll take care of the "blending" in the shader
            let blend_component = BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            };

            return BlendState {
                color: blend_component,
                alpha: blend_component,
            };
        }
    }

    pub fn translate_cull_mode(geometry_mode: u32) -> CullMode {
        let cull_front = (geometry_mode & RSPGeometry::G_CULL_FRONT as u32) != 0;
        let cull_back = (geometry_mode & RSPGeometry::G_CULL_BACK as u32) != 0;

        if cull_front && cull_back {
            CullMode::FrontAndBack
        } else if cull_front {
            CullMode::Front
        } else if cull_back {
            CullMode::Back
        } else {
            CullMode::None
        }
    }

    pub fn update_render_state(
        &mut self,
        gfx_context: &GraphicsContext,
        geometry_mode: u32,
        vertices: &[&StagingVertex; 3],
    ) {
        let depth_test = geometry_mode & RSPGeometry::G_ZBUFFER as u32 != 0;
        if depth_test != self.rendering_state.depth_test {
            self.flush(gfx_context);
            gfx_context.api.set_depth_test(depth_test);
            self.rendering_state.depth_test = depth_test;
        }

        let cull_mode = RDP::translate_cull_mode(geometry_mode);
        if cull_mode != self.rendering_state.cull_mode {
            self.flush(gfx_context);
            gfx_context.api.set_cull_mode(cull_mode);
            self.rendering_state.cull_mode = cull_mode;
        }

        let blend_state = self.translate_blend_mode(gfx_context, self.other_mode_l);

        // TODO: split checks into updating blend state separately: enable, blendeq and blendfunc
        if blend_state != self.rendering_state.blend_state {
            self.flush(gfx_context);
            gfx_context.api.set_blend_state(blend_state);
            self.rendering_state.blend_state = blend_state;
        }

        if self.viewport_or_scissor_changed {
            let viewport = self.viewport;
            if viewport != self.rendering_state.viewport {
                self.flush(gfx_context);
                gfx_context.api.set_viewport(
                    viewport.x as i32,
                    viewport.y as i32,
                    viewport.width as i32,
                    viewport.height as i32,
                );
                self.rendering_state.viewport = viewport;
            }
            let scissor = self.scissor;
            if scissor != self.rendering_state.scissor {
                self.flush(gfx_context);
                gfx_context.api.set_scissor(
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

    pub fn add_to_buf_vbo(&mut self, data: f32) {
        self.buf_vbo[self.buf_vbo_len] = data;
        self.buf_vbo_len += 1;
    }

    // MARK: - Helpers

    fn blend_component_state_none(component: BlendComponent) -> bool {
        component.src_factor == BlendFactor::One
            && component.dst_factor == BlendFactor::Zero
            && component.operation == BlendOperation::Add
    }

    pub fn scaled_x(&self) -> f32 {
        self.output_dimensions.width as f32 / SCREEN_WIDTH
    }

    pub fn scaled_y(&self) -> f32 {
        self.output_dimensions.height as f32 / SCREEN_HEIGHT
    }
}

// MARK: - C Bridge

#[no_mangle]
pub extern "C" fn RDPSetOutputDimensions(rcp: Option<&mut RCP>, dimensions: OutputDimensions) {
    let rcp = rcp.unwrap();
    rcp.rdp.output_dimensions = dimensions;
}

#[no_mangle]
pub extern "C" fn RDPLookupOrCreateShaderProgram(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    shader_id: u32,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    rcp.rdp
        .lookup_or_create_shader_program(gfx_context, shader_id);
}

#[no_mangle]
pub extern "C" fn RDPFlush(rcp: Option<&mut RCP>, gfx_context: Option<&mut GraphicsContext>) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    rcp.rdp.flush(gfx_context);
}
