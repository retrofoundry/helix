use super::gbi::defines::Viewport;

const SCREEN_WIDTH: f32 = 320.0;
const SCREEN_HEIGHT: f32 = 240.0;

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
}

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
    pub depth_test: bool,
    pub depth_mask: bool,
    pub decal_mode: bool,
    pub alpha_blend: bool,
    pub viewport: Rect,
    pub scissor: Rect,
    // shader program
}

pub struct RDP {
    pub viewport: Rect,
    pub output_dimensions: OutputDimensions,

    pub viewport_or_scissor_changed: bool,
}

impl RDP {
    pub fn new() -> Self {
        RDP {
            viewport: Rect::ZERO,
            output_dimensions: OutputDimensions::ZERO,

            viewport_or_scissor_changed: false,
        }
    }

    pub fn reset(&mut self) {}

    pub fn calculate_and_set_viewport(&mut self, viewport: Viewport) {
        let mut width = 2.0 * viewport.vscale[0] as f32 / 4.0;
        let mut height = 2.0 * viewport.vscale[1] as f32 / 4.0;
        let mut x = viewport.vtrans[0] as f32 / 4.0 - width / 2.0;
        let mut y = viewport.vtrans[1] as f32 / 4.0 - height / 2.0;

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

    // MARK: - Helpers

    fn scaled_x(&self) -> f32 {
        self.viewport.width as f32 / SCREEN_WIDTH
    }

    fn scaled_y(&self) -> f32 {
        self.viewport.height as f32 / SCREEN_HEIGHT
    }
}
