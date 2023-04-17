use super::{f3dex2::F3DEX2, GBIDefinition, GBI};

pub enum F3DZEX2 {}

impl GBIDefinition for F3DZEX2 {
    fn setup(gbi: &mut GBI) {
        F3DEX2::setup(gbi);
    }
}
