use crate::fast3d::{graphics::GraphicsContext, rdp::RDP, rsp::RSP};

use super::{
    defines::{Gfx, G_TEXRECT, G_TEXRECTFLIP},
    f3dex2::F3DEX2,
    utils::get_cmd,
    GBIDefinition, GBIResult, GBI,
};

pub struct F3DEX2E;

impl GBIDefinition for F3DEX2E {
    fn setup(gbi: &mut GBI) {
        F3DEX2::setup(gbi);
        gbi.register(G_TEXRECT as usize, F3DEX2E::gdp_texrect);
        gbi.register(G_TEXRECTFLIP as usize, F3DEX2E::gdp_texrect);
    }
}

impl F3DEX2E {
    pub fn gdp_texrect(
        _rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        mut command: *mut Gfx,
    ) -> GBIResult {
        let w0 = unsafe { (*command).words.w0 };
        let w1 = unsafe { (*command).words.w1 };

        let lrx = get_cmd(w0, 0, 24) << 8 >> 8;
        let lry = get_cmd(w1, 0, 24) << 8 >> 8;
        let tile = get_cmd(w1, 24, 3);

        command = unsafe { command.add(1) };
        let w0 = unsafe { (*command).words.w0 };
        let w1 = unsafe { (*command).words.w1 };

        let ulx = get_cmd(w0, 0, 24) << 8 >> 8;
        let uls = get_cmd(w1, 16, 16);
        let ult = get_cmd(w1, 0, 16);

        command = unsafe { command.add(1) };
        let w0 = unsafe { (*command).words.w0 };
        let w1 = unsafe { (*command).words.w1 };

        let uly = get_cmd(w0, 0, 24) << 8 >> 8;
        let dsdx = get_cmd(w1, 16, 16);
        let dtdy = get_cmd(w1, 0, 16);

        // TODO: Call texture_rectangle

        return GBIResult::Continue;
    }
}
