use crate::fast3d::rsp::StagingVertex;

use super::super::{
    rcp::RCP,
    rdp::{OutputDimensions, Rect},
};
use super::f3dex2::F3DEX2;

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMatrix(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_matrix(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPPopMatrix(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_pop_matrix(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPVertex(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_vertex(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMoveWord(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_moveword(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMoveMem(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_movemem(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPTexture(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_texture(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPGeometryMode(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_geometry_mode(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetOtherModeL(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gdp_set_other_mode_l(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetOtherModeH(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gdp_set_other_mode_h(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetScissor(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gdp_set_scissor(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetCombine(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gdp_set_combine(
        &mut rcp.rdp,
        &mut rcp.rsp,
        rcp.gfx_device.as_ref().unwrap(),
        w0,
        w1,
    );
}
