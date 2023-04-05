use super::super::{rdp::RDP, rsp::RSP};
use super::utils::{get_c0, get_segmented_address};
use super::{GBIDefinition, GBIResult, GBI};

pub enum F3DEX2 {
    // DMA
    G_VTX = 0x01,
    G_MODIFYVTX = 0x02,
    G_CULLDL = 0x03,
    G_BRANCH_Z = 0x04,
    G_TRI1 = 0x05,
    G_TRI2 = 0x06,
    G_QUAD = 0x07,
    G_LINE3D = 0x08,

    G_TEXTURE = 0xD7,
    G_POPMTX = 0xD8,
    G_GEOMETRYMODE = 0xD9,
    G_MTX = 0xDA,
    G_LOAD_UCODE = 0xDD,
    G_DL = 0xDE,
    G_ENDDL = 0xDF,

    // RDP
    G_SETCIMG = 0xFF,
    G_SETZIMG = 0xFE,
    G_SETTIMG = 0xFD,
    G_SETCOMBINE = 0xFC,
    G_SETENVCOLOR = 0xFB,
    G_SETPRIMCOLOR = 0xFA,
    G_SETBLENDCOLOR = 0xF9,
    G_SETFOGCOLOR = 0xF8,
    G_SETFILLCOLOR = 0xF7,
    G_FILLRECT = 0xF6,
    G_SETTILE = 0xF5,
    G_LOADTILE = 0xF4,
    G_LOADBLOCK = 0xF3,
    G_SETTILESIZE = 0xF2,
    G_LOADTLUT = 0xF0,
    G_RDPSETOTHERMODE = 0xEF,
    G_SETPRIMDEPTH = 0xEE,
    G_SETSCISSOR = 0xED,
    G_SETCONVERT = 0xEC,
    G_SETKEYR = 0xEB,
    G_SETKEYFB = 0xEA,
    G_RDPFULLSYNC = 0xE9,
    G_RDPTILESYNC = 0xE8,
    G_RDPPIPESYNC = 0xE7,
    G_RDPLOADSYNC = 0xE6,
    G_TEXRECTFLIP = 0xE5,
    G_TEXRECT = 0xE4,
    G_SETOTHERMODE_H = 0xE3,
    G_SETOTHERMODE_L = 0xE2,
}

impl GBIDefinition for F3DEX2 {
    fn setup(gbi: &mut GBI) {
        gbi.register(F3DEX2::G_GEOMETRYMODE as usize, F3DEX2::gsp_geometry_mode);
        gbi.register(F3DEX2::G_DL as usize, F3DEX2::sub_dl);
        gbi.register(F3DEX2::G_ENDDL as usize, |_, _, _, _| { GBIResult::Return });
    }
}

impl F3DEX2 {
    pub fn gsp_geometry_mode(_rdp: &mut RDP, rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        let clear_bits = get_c0(w0, 0, 24);
        let set_bits = w1;

        rsp.geometry_mode &= !clear_bits as u32;
        rsp.geometry_mode |= set_bits as u32;
        rsp.state_changed = true;

        GBIResult::Continue
    }

    pub fn sub_dl(_rdp: &mut RDP, _rsp: &mut RSP, w0: usize, w1: usize) -> GBIResult {
        if get_c0(w0, 16, 1) == 0 {
            // Push return address
            let new_addr = get_segmented_address(w1);
            return GBIResult::Recurse(new_addr);
        } else {
            let new_addr = get_segmented_address(w1);
            return GBIResult::SetAddress(new_addr);
        }
    }
}
