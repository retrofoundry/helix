use crate::fast3d::graphics::GraphicsContext;

use super::super::rcp::RCP;
use super::f3dex2::F3DEX2;

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMatrix(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gsp_matrix(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPPopMatrix(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gsp_pop_matrix(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPVertex(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gsp_vertex(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMoveWord(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gsp_moveword(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMoveMem(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gsp_movemem(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPTexture(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gsp_texture(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPGeometryMode(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gsp_geometry_mode(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetOtherModeL(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_other_mode_l(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetOtherModeH(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_other_mode_h(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetScissor(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_scissor(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetCombine(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_combine(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetTile(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_tile(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPLoadBlock(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_load_block(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPLoadTile(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_load_tile(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetTileSize(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_tile_size(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetTextureImage(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_texture_image(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPLoadTLUT(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_load_tlut(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetEnvColor(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_env_color(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetPrimColor(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_prim_color(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetFogColor(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_fog_color(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetFillColor(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_fill_color(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetDepthImage(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_depth_image(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GDPSetColorImage(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    w0: usize,
    w1: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    F3DEX2::gdp_set_color_image(&mut rcp.rdp, &mut rcp.rsp, gfx_context, w0, w1);
}