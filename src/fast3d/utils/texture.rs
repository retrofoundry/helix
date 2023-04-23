use super::super::{graphics::GraphicsContext, rcp::RCP};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ImageFormat {
    G_IM_FMT_RGBA = 0x00,
    G_IM_FMT_YUV = 0x01,
    G_IM_FMT_CI = 0x02,
    G_IM_FMT_IA = 0x03,
    G_IM_FMT_I = 0x04,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ImageSize {
    G_IM_SIZ_4b = 0x00,
    G_IM_SIZ_8b = 0x01,
    G_IM_SIZ_16b = 0x02,
    G_IM_SIZ_32b = 0x03,
}

pub struct TextureManager {
    pub map: HashMap<usize, Texture>,
    pub lru: VecDeque<usize>,
    pub capacity: usize,
}

impl TextureManager {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            lru: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn lookup(
        &mut self,
        gfx_context: &GraphicsContext,
        tile: i32,
        orig_addr: usize,
        fmt: u8,
        siz: u8,
    ) -> Option<&mut Texture> {
        if let Some(value) = self.map.get_mut(&orig_addr) {
            if value.fmt == fmt && value.size == siz {
                gfx_context.api.select_texture(tile, value.texture_id);
                self.lru.retain(|&k| k != orig_addr);
                self.lru.push_back(orig_addr);
                return Some(value);
            }
        }
        None
    }

    pub fn insert_if_not_found(
        &mut self,
        gfx_context: &GraphicsContext,
        tile: i32,
        orig_addr: usize,
        fmt: u8,
        siz: u8,
    ) -> &mut Texture {
        if self.map.len() == self.capacity {
            if let Some(lru_key) = self.lru.pop_front() {
                self.map.remove(&lru_key);
                // TODO: Remove texture from gfx_device
            }
        }
        let texture_id = gfx_context.api.new_texture();
        gfx_context.api.select_texture(tile, texture_id);
        gfx_context.api.set_sampler_parameters(tile, false, 0, 0);
        let value = self.map.entry(orig_addr).or_insert(Texture {
            texture_addr: orig_addr,
            fmt,
            size: siz,
            texture_id,
            cms: 0,
            cmt: 0,
            linear_filter: false,
        });
        self.lru.push_back(orig_addr);
        value
    }
}

pub struct TextureState {
    pub on: bool,
    /// Index of parameter-setting tile descriptor (3bit precision, 0 - 7)
    pub tile: u8,
    pub level: u8,
    pub scale_s: u16,
    pub scale_t: u16,
}

impl TextureState {
    pub const EMPTY: Self = Self {
        on: false,
        tile: 0,
        level: 0,
        scale_s: 0,
        scale_t: 0,
    };

    pub fn new(on: bool, tile: u8, level: u8, scale_s: u16, scale_t: u16) -> Self {
        Self {
            on,
            tile,
            level,
            scale_s,
            scale_t,
        }
    }
}

pub struct TextureImageState {
    pub format: u8,
    pub size: u8,
    pub width: u16,
    pub address: usize,
}

impl TextureImageState {
    pub const EMPTY: Self = Self {
        format: 0,
        size: 0,
        width: 0,
        address: 0,
    };

    pub fn new(format: u8, size: u8, width: u16, address: usize) -> Self {
        Self {
            format,
            size,
            width,
            address,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Texture {
    texture_addr: usize,
    fmt: u8,
    size: u8,

    texture_id: u32,
    cms: u8,
    cmt: u8,

    linear_filter: bool,
}

impl Texture {
    pub const EMPTY: Self = Self {
        texture_addr: 0,
        fmt: 0,
        size: 0,
        texture_id: 0,
        cms: 0,
        cmt: 0,
        linear_filter: false,
    };
}

#[no_mangle]
pub extern "C" fn RDPLookupTexture(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    tile: i32,
    orig_addr: *const u8,
    fmt: u8,
    siz: u8,
) -> bool {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    let texture_cache = &mut rcp.rdp.texture_manager;
    if let Some(value) = texture_cache.lookup(gfx_context, tile, orig_addr as usize, fmt, siz) {
        rcp.rdp.rendering_state.textures[tile as usize] = *value;
        true
    } else {
        let value =
            texture_cache.insert_if_not_found(gfx_context, tile, orig_addr as usize, fmt, siz);
        rcp.rdp.rendering_state.textures[tile as usize] = *value;
        false
    }
}
