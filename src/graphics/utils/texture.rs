use std::collections::{HashMap, VecDeque};

use super::super::{gfx_device::GfxDevice, rcp::RCP};

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
        gfx_device: &GfxDevice,
        tile: i32,
        orig_addr: usize,
        fmt: u8,
        siz: u8,
    ) -> Option<&mut Texture> {
        if let Some(value) = self.map.get_mut(&orig_addr) {
            if value.fmt == fmt && value.size == siz {
                gfx_device.select_texture(tile, value.texture_id);
                self.lru.retain(|&k| k != orig_addr);
                self.lru.push_back(orig_addr);
                return Some(value);
            }
        }
        None
    }

    pub fn insert_if_not_found(
        &mut self,
        gfx_device: &GfxDevice,
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
        let texture_id = gfx_device.new_texture();
        gfx_device.select_texture(tile, texture_id);
        gfx_device.set_sampler_parameters(tile, false, 0, 0);
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

#[no_mangle]
pub extern "C" fn RSPLookupTexture(
    rcp: Option<&mut RCP>,
    tile: i32,
    orig_addr: *const u8,
    fmt: u8,
    siz: u8,
    output: *mut *mut Texture,
) -> bool {
    let rcp = rcp.unwrap();
    let gfx_device = rcp.gfx_device.as_ref().unwrap();
    let texture_cache = &mut rcp.rsp.texture_manager;
    if let Some(value) = texture_cache.lookup(gfx_device, tile, orig_addr as usize, fmt, siz) {
        unsafe { *output = value };
        true
    } else {
        let value =
            texture_cache.insert_if_not_found(gfx_device, tile, orig_addr as usize, fmt, siz);
        unsafe { *output = value };
        false
    }
}
