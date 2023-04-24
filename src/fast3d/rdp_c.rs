// MARK: - C Bridge

use super::{
    graphics::{GraphicsContext, ShaderProgram},
    rcp::RCP,
    rdp::{OutputDimensions, Rect, TMEMMapEntry},
    utils::{color_combiner::CombineParams, texture::Texture},
};

#[no_mangle]
pub extern "C" fn RDPSetOutputDimensions(rcp: Option<&mut RCP>, dimensions: OutputDimensions) {
    let rcp = rcp.unwrap();
    rcp.rdp.output_dimensions = dimensions;
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

#[no_mangle]
pub extern "C" fn RDPGetScissorPtr(rcp: Option<&mut RCP>) -> *mut Rect {
    let rcp = rcp.unwrap();
    &mut rcp.rdp.scissor as *mut Rect
}

#[no_mangle]
pub extern "C" fn RDPFlush(rcp: Option<&mut RCP>, gfx_context: Option<&mut GraphicsContext>) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    rcp.rdp.flush(gfx_context);
}

#[no_mangle]
pub extern "C" fn RDPLookupOrCreateColorCombiner(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    cc_id: u32,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    rcp.rdp.lookup_or_create_color_combiner(gfx_context, cc_id);
}

#[no_mangle]
pub extern "C" fn RDPAddToVBOAndIncrement(rcp: Option<&mut RCP>, value: f32) {
    let rcp = rcp.unwrap();
    rcp.rdp.buf_vbo[rcp.rdp.buf_vbo_len] = value;
    rcp.rdp.buf_vbo_len += 1;
}

#[no_mangle]
pub extern "C" fn RDPIncrementTriangleCountAndReturn(rcp: Option<&mut RCP>) -> usize {
    let rcp = rcp.unwrap();
    rcp.rdp.buf_vbo_num_tris += 1;
    rcp.rdp.buf_vbo_num_tris
}

#[no_mangle]
pub extern "C" fn RDPSetRenderingStateViewport(rcp: Option<&mut RCP>, viewport: Rect) {
    let rcp = rcp.unwrap();
    rcp.rdp.rendering_state.viewport = viewport;
}

#[no_mangle]
pub extern "C" fn RDPSetRenderingStateScissor(rcp: Option<&mut RCP>, scissor: Rect) {
    let rcp = rcp.unwrap();
    rcp.rdp.rendering_state.scissor = scissor;
}

#[no_mangle]
pub extern "C" fn RDPLookupOrCreateShaderProgram(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    shader_id: u32,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    rcp.rdp
        .lookup_or_create_shader_program(gfx_context, shader_id);
}

#[no_mangle]
pub extern "C" fn RDPGetRenderingStateShaderProgram(rcp: Option<&mut RCP>) -> *const ShaderProgram {
    let rcp = rcp.unwrap();
    rcp.rdp.rendering_state.shader_program
}

#[no_mangle]
pub extern "C" fn RDPSetRenderingStateShaderProgram(
    rcp: Option<&mut RCP>,
    prg: *mut ShaderProgram,
) {
    let rcp = rcp.unwrap();
    rcp.rdp.rendering_state.shader_program = prg;
}

#[no_mangle]
pub extern "C" fn RDPGetRenderingStateTextureAtIndex(
    rcp: Option<&mut RCP>,
    index: usize,
) -> *const Texture {
    let rcp = rcp.unwrap();
    Box::into_raw(Box::new(rcp.rdp.rendering_state.textures[index]))
}

#[no_mangle]
pub extern "C" fn RDPViewportDoesNotEqualRenderingStateViewport(rcp: Option<&mut RCP>) -> bool {
    let rcp = rcp.unwrap();
    rcp.rdp.rendering_state.viewport != rcp.rdp.viewport
}

#[no_mangle]
pub extern "C" fn RDPScissorDoesNotEqualRenderingStateScissor(rcp: Option<&mut RCP>) -> bool {
    let rcp = rcp.unwrap();
    rcp.rdp.rendering_state.scissor != rcp.rdp.scissor
}

#[no_mangle]
pub extern "C" fn RDPGetOtherModeL(rcp: Option<&mut RCP>) -> u32 {
    let rcp = rcp.unwrap();
    rcp.rdp.other_mode_l
}

#[no_mangle]
pub extern "C" fn RDPGetOtherModeH(rcp: Option<&mut RCP>) -> u32 {
    let rcp = rcp.unwrap();
    rcp.rdp.other_mode_h
}

#[no_mangle]
pub extern "C" fn RDPSetOtherModeH(rcp: Option<&mut RCP>, value: u32) {
    let rcp = rcp.unwrap();
    rcp.rdp.other_mode_h = value;
}

#[no_mangle]
pub extern "C" fn RDPGetCombineU32(rcp: Option<&mut RCP>) -> u32 {
    let rcp = rcp.unwrap();
    rcp.rdp.combine.to_u32()
}

#[no_mangle]
pub extern "C" fn RDPGetCombine(rcp: Option<&mut RCP>) -> *const CombineParams {
    let rcp = rcp.unwrap();
    Box::into_raw(Box::new(rcp.rdp.combine))
}

#[no_mangle]
pub extern "C" fn RDPSetCombine(rcp: Option<&mut RCP>, value: *mut CombineParams) {
    let rcp = rcp.unwrap();
    rcp.rdp.combine = unsafe { *value };
}

#[no_mangle]
pub extern "C" fn RDPUpdateRenderState(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    vertex_id1: u8,
    vertex_id2: u8,
    vertex_id3: u8,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();

    let vertex1 = &rcp.rsp.vertex_table[vertex_id1 as usize];
    let vertex2 = &rcp.rsp.vertex_table[vertex_id2 as usize];
    let vertex3 = &rcp.rsp.vertex_table[vertex_id3 as usize];
    let vertex_array = [vertex1, vertex2, vertex3];

    rcp.rdp
        .update_render_state(gfx_context, rcp.rsp.geometry_mode, &vertex_array);
}

#[no_mangle]
pub extern "C" fn RDPGetTextureChangedAtIndex(rcp: Option<&mut RCP>, index: u8) -> bool {
    let rcp = rcp.unwrap();
    rcp.rdp.textures_changed[index as usize]
}

#[no_mangle]
pub extern "C" fn RDPSetTextureChangedAtIndex(rcp: Option<&mut RCP>, index: u8, value: bool) {
    let rcp = rcp.unwrap();
    rcp.rdp.textures_changed[index as usize] = value;
}

#[no_mangle]
pub extern "C" fn RDPGetTileDescriptorTMEM(rcp: Option<&mut RCP>, index: u8) -> u16 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[index as usize].tmem
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorULS(rcp: Option<&mut RCP>) -> u16 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].uls
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorULT(rcp: Option<&mut RCP>) -> u16 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].ult
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorLRS(rcp: Option<&mut RCP>) -> u16 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].lrs
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorLRT(rcp: Option<&mut RCP>) -> u16 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].lrt
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorCMS(rcp: Option<&mut RCP>) -> u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].cm_s
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorCMT(rcp: Option<&mut RCP>) -> u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].cm_t
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorFormat(rcp: Option<&mut RCP>) -> u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].format as u8
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorSize(rcp: Option<&mut RCP>) -> u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].size as u8
}

#[no_mangle]
pub extern "C" fn RDPGetCurrentTileDescriptorLineSizeBytes(rcp: Option<&mut RCP>) -> u32 {
    let rcp = rcp.unwrap();
    rcp.rdp.tile_descriptors[rcp.rdp.texture_state.tile as usize].line as u32 * 8
}

#[no_mangle]
pub extern "C" fn RDPSetTMEMMap(rcp: Option<&mut RCP>, tile_number: u8, address: *const u8) {
    let rcp = rcp.unwrap();
    rcp.rdp
        .tmem_map
        .insert(tile_number as u16, TMEMMapEntry::new(address as usize));
}

#[no_mangle]
pub extern "C" fn RDPGetTMEMMapEntryAddress(rcp: Option<&mut RCP>, tile_number: u8) -> *const u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.tmem_map.get(&(tile_number as u16)).unwrap().address as *const u8
}

#[no_mangle]
pub extern "C" fn RDPGetTextureImageStateAddress(rcp: Option<&mut RCP>) -> *const u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.texture_image_state.address as *const u8
}

#[no_mangle]
pub extern "C" fn RDPGetTextureImageStateSize(rcp: Option<&mut RCP>) -> u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.texture_image_state.size
}

#[no_mangle]
pub extern "C" fn RDPPaletteAtTMEMIndex(rcp: Option<&mut RCP>, index: u8) -> *const u8 {
    let rcp = rcp.unwrap();
    rcp.rdp
        .tmem_map
        .get(&(u16::MAX - index as u16))
        .unwrap()
        .address as *const u8
}

#[no_mangle]
pub extern "C" fn RDPImportTileTexture(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    tile: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    rcp.rdp.import_tile_texture(gfx_context, tile);
}

#[no_mangle]
pub extern "C" fn RDPFlushTextures(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();
    rcp.rdp.flush_textures(gfx_context);
}

#[no_mangle]
pub extern "C" fn RDPGetFogColor(rcp: Option<&mut RCP>) -> *const u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.fog_color.as_ptr()
}

#[no_mangle]
pub extern "C" fn RDPGetFillColor(rcp: Option<&mut RCP>) -> *const u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.fill_color.as_ptr()
}

#[no_mangle]
pub extern "C" fn RDPGetPrimColor(rcp: Option<&mut RCP>) -> *const u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.prim_color.as_ptr()
}

#[no_mangle]
pub extern "C" fn RDPGetEnvColor(rcp: Option<&mut RCP>) -> *const u8 {
    let rcp = rcp.unwrap();
    rcp.rdp.env_color.as_ptr()
}

#[no_mangle]
pub extern "C" fn RDPGetDepthImage(rcp: Option<&mut RCP>) -> usize {
    let rcp = rcp.unwrap();
    rcp.rdp.depth_image
}

#[no_mangle]
pub extern "C" fn RDPGetColorImage(rcp: Option<&mut RCP>) -> usize {
    let rcp = rcp.unwrap();
    rcp.rdp.color_image
}
