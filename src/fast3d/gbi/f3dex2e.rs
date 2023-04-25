use crate::fast3d::{rdp::RDP, rsp::RSP, graphics::GraphicsContext};

use super::{GBIDefinition, GBI, f3dex2::F3DEX2, defines::G_TEXRECT, GBIResult, utils::get_cmd};

pub struct F3DEX2E;

impl GBIDefinition for F3DEX2E {
    fn setup(gbi: &mut GBI) {
        F3DEX2::setup(gbi);
        // gbi.register(G_TEXRECT as usize, F3DEX2E::gdp_texrect);
        // gbi.register(G_TEXRECT as usize, F3DEX2E::gdp_texrect_flip);
    }
}

impl F3DEX2E {
    pub fn gdp_texrect(
        _rdp: &mut RDP,
        rsp: &mut RSP,
        _gfx_context: &GraphicsContext,
        w0: usize,
        w1: usize,
    ) -> GBIResult {
        // int32_t lrx, lry, tile, ulx, uly;
        //         uint32_t uls, ult, dsdx, dtdy;
        //         lrx = (int32_t)(C0(0, 24) << 8) >> 8;
        //         lry = (int32_t)(C1(0, 24) << 8) >> 8;
        //         tile = (int32_t)C1(24, 3);
        //         ++cmd;

        //         ulx = (int32_t)(C0(0, 24) << 8) >> 8;
        //         uls = (uint16_t)C1(16, 16);
        //         ult = (uint16_t)C1(0, 16);
        //         ++cmd;

        //         uly = (int32_t)(C0(0, 24) << 8) >> 8;
        //         dsdx = (uint16_t)C1(16, 16);
        //         dtdy = (uint16_t)C1(0, 16);

        let lrx = get_cmd(w0, 0, 24) << 8 >> 8;
        let lry = get_cmd(w1, 0, 24) << 8 >> 8;

        return GBIResult::Continue;
    }
}
