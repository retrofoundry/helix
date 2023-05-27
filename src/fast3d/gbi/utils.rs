use wgpu::BlendFactor;

use crate::fast3d::utils::color_combiner::{CombineParams, ACMUX, CCMUX, SHADER};
use crate::fast3d::{
    graphics::CullMode,
    rdp::{
        AlphaCompare, BlendParamB, BlendParamPMColor, OtherModeHCycleType, OtherModeH_Layout,
        OtherModeLayoutL,
    },
    rsp::RSPGeometry,
    utils::texture::TextFilt,
};

pub fn get_cmd(val: usize, start_bit: u32, num_bits: u32) -> usize {
    (val >> start_bit) & ((1 << num_bits) - 1)
}

pub fn get_segmented_address(w1: usize) -> usize {
    w1
}

pub fn other_mode_l_uses_texture_edge(other_mode_l: u32) -> bool {
    other_mode_l >> (OtherModeLayoutL::CVG_X_ALPHA as u32) & 0x01 == 0x01
}

pub fn other_mode_l_uses_alpha(other_mode_l: u32) -> bool {
    other_mode_l & ((BlendParamB::G_BL_A_MEM as u32) << (OtherModeLayoutL::B_1 as u32)) == 0
}

pub fn other_mode_l_uses_fog(other_mode_l: u32) -> bool {
    (other_mode_l >> OtherModeLayoutL::P_1 as u32) == BlendParamPMColor::G_BL_CLR_FOG as u32
}

pub fn other_mode_l_uses_noise(other_mode_l: u32) -> bool {
    other_mode_l & AlphaCompare::G_AC_DITHER as u32 == AlphaCompare::G_AC_DITHER as u32
}

pub fn get_cycle_type_from_other_mode_h(mode_h: u32) -> OtherModeHCycleType {
    match (mode_h >> OtherModeH_Layout::G_MDSFT_CYCLETYPE as u32) & 0x03 {
        x if x == OtherModeHCycleType::G_CYC_1CYCLE as u32 => OtherModeHCycleType::G_CYC_1CYCLE,
        x if x == OtherModeHCycleType::G_CYC_2CYCLE as u32 => OtherModeHCycleType::G_CYC_2CYCLE,
        x if x == OtherModeHCycleType::G_CYC_COPY as u32 => OtherModeHCycleType::G_CYC_COPY,
        x if x == OtherModeHCycleType::G_CYC_FILL as u32 => OtherModeHCycleType::G_CYC_FILL,
        _ => panic!("Invalid cycle type"),
    }
}

pub fn get_textfilter_from_other_mode_h(mode_h: u32) -> TextFilt {
    match (mode_h >> OtherModeH_Layout::G_MDSFT_TEXTFILT as u32) & 0x3 {
        x if x == TextFilt::G_TF_POINT as u32 => TextFilt::G_TF_POINT,
        x if x == TextFilt::G_TF_AVERAGE as u32 => TextFilt::G_TF_AVERAGE,
        x if x == TextFilt::G_TF_BILERP as u32 => TextFilt::G_TF_BILERP,
        _ => panic!("Invalid text filter"),
    }
}

pub fn translate_blend_param_b(param: u32, src: BlendFactor) -> BlendFactor {
    match param {
        x if x == BlendParamB::G_BL_1MA as u32 => {
            if src == BlendFactor::SrcAlpha {
                BlendFactor::OneMinusSrcAlpha
            } else if src == BlendFactor::One {
                BlendFactor::Zero
            } else {
                BlendFactor::One
            }
        }
        x if x == BlendParamB::G_BL_A_MEM as u32 => BlendFactor::DstAlpha,
        x if x == BlendParamB::G_BL_1 as u32 => BlendFactor::One,
        x if x == BlendParamB::G_BL_0 as u32 => BlendFactor::Zero,
        _ => panic!("Unknown Blend Param B: {}", param),
    }
}

pub fn translate_cull_mode(geometry_mode: u32) -> Option<wgpu::Face> {
    let cull_front = (geometry_mode & RSPGeometry::G_CULL_FRONT as u32) != 0;
    let cull_back = (geometry_mode & RSPGeometry::G_CULL_BACK as u32) != 0;

    if cull_front && cull_back {
        panic!("Culling both front and back faces is not supported");
    } else if cull_front {
        Some(wgpu::Face::Front)
    } else if cull_back {
        Some(wgpu::Face::Back)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub trait I32MathExt {
        fn ushr(self, n: u32) -> u32;
    }

    impl I32MathExt for i32 {
        fn ushr(self, n: u32) -> u32 {
            ((self >> n) & ((1 << (32 - n)) - 1)) as u32
        }
    }

    #[test]
    fn test_get_cmd() {
        let word: usize = 84939284;
        let a = get_cmd(word, 16, 8) / 2;
        let b = get_cmd(word, 8, 8) / 2;
        let c = get_cmd(word, 0, 8) / 2;

        assert_eq!(a, 8);
        assert_eq!(b, 9);
        assert_eq!(c, 10);

        assert_eq!(a, ((((word as i32).ushr(16)) & 0xFF) / 2) as usize);
    }
}
