use crate::graphics::rsp::StagingVertex;

use super::super::{
    rcp::RCP,
    rdp::{OutputDimensions, Rect},
};
use super::{defines::Light, f3dex2::F3DEX2};

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMatrix(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_matrix(&mut rcp.rdp, &mut rcp.rsp, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPPopMatrix(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_pop_matrix(&mut rcp.rdp, &mut rcp.rsp, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPVertex(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_vertex(&mut rcp.rdp, &mut rcp.rsp, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMoveWord(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_moveword(&mut rcp.rdp, &mut rcp.rsp, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPMoveMem(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_movemem(&mut rcp.rdp, &mut rcp.rsp, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPTexture(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_texture(&mut rcp.rdp, &mut rcp.rsp, w0, w1);
}

#[no_mangle]
pub extern "C" fn F3DEX2_GSPGeometryMode(rcp: Option<&mut RCP>, w0: usize, w1: usize) {
    let rcp = rcp.unwrap();
    F3DEX2::gsp_geometry_mode(&mut rcp.rdp, &mut rcp.rsp, w0, w1);
}

// RSP Getters and Setters

#[no_mangle]
pub extern "C" fn RSPGetGeometryMode(rcp: Option<&mut RCP>) -> u32 {
    let rcp = rcp.unwrap();
    return rcp.rsp.geometry_mode;
}

#[no_mangle]
pub extern "C" fn RSPSetGeometryMode(rcp: Option<&mut RCP>, value: u32) {
    let rcp = rcp.unwrap();
    rcp.rsp.geometry_mode = value;
}

#[no_mangle]
pub extern "C" fn RSPGetLightsValid(rcp: Option<&mut RCP>) -> bool {
    let rcp = rcp.unwrap();
    rcp.rsp.lights_valid
}

#[no_mangle]
pub extern "C" fn RSPSetLightsValid(rcp: Option<&mut RCP>, value: bool) {
    let rcp = rcp.unwrap();
    rcp.rsp.lights_valid = value;
}

#[no_mangle]
pub extern "C" fn RSPGetNumLights(rcp: Option<&mut RCP>) -> u8 {
    let rcp = rcp.unwrap();
    rcp.rsp.num_lights
}

#[no_mangle]
pub extern "C" fn RSPGetFogMultiplier(rcp: Option<&mut RCP>) -> i16 {
    let rcp = rcp.unwrap();
    rcp.rsp.fog_multiplier
}

#[no_mangle]
pub extern "C" fn RSPGetFogOffset(rcp: Option<&mut RCP>) -> i16 {
    let rcp = rcp.unwrap();
    rcp.rsp.fog_offset
}

#[no_mangle]
pub extern "C" fn RSPGetLightAtIndex(rcp: Option<&mut RCP>, index: usize) -> Light {
    let rcp = rcp.unwrap();
    rcp.rsp.lights[index]
}

#[no_mangle]
pub extern "C" fn RSPGetLightAtIndexPtr(rcp: Option<&mut RCP>, index: usize) -> *mut Light {
    let rcp = rcp.unwrap();
    &mut rcp.rsp.lights[index] as *mut Light
}

#[no_mangle]
pub extern "C" fn RSPGetTextureScalingFactorS(rcp: Option<&mut RCP>) -> u16 {
    let rcp = rcp.unwrap();
    rcp.rsp.texture_scaling_factor.scale_s
}

#[no_mangle]
pub extern "C" fn RSPGetTextureScalingFactorT(rcp: Option<&mut RCP>) -> u16 {
    let rcp = rcp.unwrap();
    rcp.rsp.texture_scaling_factor.scale_t
}

#[no_mangle]
pub extern "C" fn RSPGetMatrixStackPointer(rcp: Option<&mut RCP>) -> usize {
    let rcp = rcp.unwrap();
    rcp.rsp.matrix_stack_pointer
}

#[no_mangle]
pub extern "C" fn RSPGetMatrixAtIndex(rcp: Option<&mut RCP>, index: usize) -> *mut [[f32; 4]; 4] {
    let rcp = rcp.unwrap();
    &mut rcp.rsp.matrix_stack[index]
}

#[no_mangle]
pub extern "C" fn RSPGetModelViewProjectionMatrix(rcp: Option<&mut RCP>) -> *mut [[f32; 4]; 4] {
    let rcp = rcp.unwrap();
    &mut rcp.rsp.modelview_projection_matrix
}

#[no_mangle]
pub extern "C" fn RSPGetStagingVertexAtIndexPtr(rcp: Option<&mut RCP>, index: usize) -> *mut StagingVertex {
    let rcp = rcp.unwrap();
    &mut rcp.rsp.vertex_table[index] as *mut StagingVertex
}

// RDP Getters and Setters

#[no_mangle]
pub extern "C" fn RDPSetOutputDimensions(rcp: Option<&mut RCP>, dimensions: OutputDimensions) {
    let rcp = rcp.unwrap();
    rcp.rdp.output_dimensions = dimensions;
}

#[no_mangle]
pub extern "C" fn RDPGetViewportOrScissorChanged(rcp: Option<&mut RCP>) -> bool {
    let rcp = rcp.unwrap();
    rcp.rdp.viewport_or_scissor_changed
}

#[no_mangle]
pub extern "C" fn RDPSetViewportOrScissorChanged(rcp: Option<&mut RCP>, value: bool) {
    let rcp = rcp.unwrap();
    rcp.rdp.viewport_or_scissor_changed = value;
}

#[no_mangle]
pub extern "C" fn RDPGetViewport(rcp: Option<&mut RCP>) -> Rect {
    let rcp = rcp.unwrap();
    rcp.rdp.viewport
}

#[no_mangle]
pub extern "C" fn RDPGetViewportPtr(rcp: Option<&mut RCP>) -> *mut Rect {
    let rcp = rcp.unwrap();
    &mut rcp.rdp.viewport as *mut Rect
}

#[no_mangle]
pub extern "C" fn RDPSetViewport(rcp: Option<&mut RCP>, viewport: Rect) {
    let rcp = rcp.unwrap();
    rcp.rdp.viewport = viewport;
}
