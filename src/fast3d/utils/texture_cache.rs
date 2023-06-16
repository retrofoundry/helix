use std::{
    collections::{hash_map::DefaultHasher, HashMap, VecDeque},
    hash::{Hash, Hasher},
};



use crate::fast3d::graphics::GraphicsIntermediateTexture;

use super::texture::{ImageFormat, ImageSize};

pub struct TextureCache {
    pub map: HashMap<u64, GraphicsIntermediateTexture>,
    pub lru: VecDeque<u64>,
    pub capacity: usize,
}

impl TextureCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            lru: VecDeque::new(),
            capacity,
        }
    }

    pub fn contains(
        &self,
        game_address: usize,
        format: ImageFormat,
        size: ImageSize,
    ) -> Option<u64> {
        let mut hasher = DefaultHasher::new();
        game_address.hash(&mut hasher);
        format.hash(&mut hasher);
        size.hash(&mut hasher);
        let hash = hasher.finish();

        if self.map.contains_key(&hash) {
            // trace!("Texture found in decoding cache");
            return Some(hash);
        }

        None
    }

    pub fn get(&mut self, hash: u64) -> Option<&GraphicsIntermediateTexture> {
        if let Some(texture) = self.map.get(&hash) {
            self.lru.push_back(hash);
            return Some(texture);
        }

        None
    }

    pub fn get_mut(&mut self, hash: u64) -> Option<&mut GraphicsIntermediateTexture> {
        if let Some(texture) = self.map.get_mut(&hash) {
            self.lru.push_back(hash);
            return Some(texture);
        }

        None
    }

    pub fn insert(
        &mut self,
        game_address: usize,
        format: ImageFormat,
        size: ImageSize,
        width: u32,
        height: u32,
        data: Vec<u8>,
    ) -> u64 {
        if self.map.len() >= self.capacity {
            if let Some(key) = self.lru.pop_front() {
                self.map.remove(&key);
                // TODO: Keep track of removed textures so they can be deleted from the GPU
            }
        }

        let texture =
            GraphicsIntermediateTexture::new(game_address, format, size, width, height, data);

        let mut hasher = DefaultHasher::new();
        game_address.hash(&mut hasher);
        format.hash(&mut hasher);
        size.hash(&mut hasher);
        let hash = hasher.finish();

        self.map.insert(hash, texture);

        hash
    }
}
