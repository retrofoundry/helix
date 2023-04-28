use wgpu::{BlendComponent, BlendFactor, BlendOperation};

trait BlendComponentExt {
    fn is_none(&self) -> bool;
}

impl BlendComponentExt for BlendComponent {
    fn is_none(&self) -> bool {
        self.src_factor == BlendFactor::One
            && self.dst_factor == BlendFactor::Zero
            && self.operation == BlendOperation::Add
    }
}
