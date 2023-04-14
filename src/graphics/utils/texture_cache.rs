use std::collections::{HashMap, VecDeque};

use super::super::{gfx_device::GfxDevice, rcp::RCP};

pub struct TextureCache {
    pub map: HashMap<usize, TextureCacheValue>,
    pub lru: VecDeque<usize>,
    pub capacity: usize,
}

impl TextureCache {
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
        orig_addr: *const u8,
        fmt: u8,
        siz: u8,
    ) -> Option<&mut TextureCacheValue> {
        let key = orig_addr as usize;
        if let Some(value) = self.map.get_mut(&key) {
            if value.fmt == fmt && value.size == siz {
                gfx_device.select_texture(tile, value.texture_id);
                self.lru.retain(|&k| k != key);
                self.lru.push_back(key);
                return Some(value);
            }
        }
        None
    }

    pub fn insert_if_not_found(
        &mut self,
        gfx_device: &GfxDevice,
        tile: i32,
        orig_addr: *const u8,
        fmt: u8,
        siz: u8,
    ) -> &mut TextureCacheValue {
        let key = orig_addr as usize;
        if self.map.len() == self.capacity {
            if let Some(lru_key) = self.lru.pop_front() {
                self.map.remove(&lru_key);
                // TODO: Remove texture from gfx_device
            }
        }
        let texture_id = gfx_device.new_texture();
        gfx_device.select_texture(tile, texture_id);
        gfx_device.set_sampler_parameters(tile, false, 0, 0);
        let value = self.map.entry(key).or_insert(TextureCacheValue {
            texture_addr: key,
            fmt,
            size: siz,
            texture_id,
            cms: 0,
            cmt: 0,
            linear_filter: false,
        });
        self.lru.push_back(key);
        value
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TextureCacheValue {
    texture_addr: usize,
    fmt: u8,
    size: u8,

    texture_id: u32,
    cms: u8,
    cmt: u8,

    linear_filter: bool,
}

#[no_mangle]
pub extern "C" fn TextureCacheLookup(
    rcp: *mut RCP,
    tile: i32,
    orig_addr: *const u8,
    fmt: u8,
    siz: u8,
    output: *mut *mut TextureCacheValue,
) -> bool {
    let rcp = unsafe { &mut *rcp };
    let gfx_device = rcp.gfx_device.as_ref().unwrap();
    let texture_cache = &mut rcp.rsp.texture_cache;
    if let Some(value) = texture_cache.lookup(gfx_device, tile, orig_addr, fmt, siz) {
        unsafe { *output = value };
        true
    } else {
        let value = texture_cache.insert_if_not_found(gfx_device, tile, orig_addr, fmt, siz);
        unsafe { *output = value };
        false
    }
}
