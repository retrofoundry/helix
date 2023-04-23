// TODO: Remove. Bridge to test Rust implementation against working copy.

#ifndef HELIX_LIB_RCP_H
#define HELIX_LIB_RCP_H

#include <stdint.h>
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

struct ShaderProgram;
struct WGPUBlendState;

struct CGraphicsDevice {
    bool (*z_is_from_0_to_1)(void);
    void (*unload_shader)(struct ShaderProgram *old_prg);
    void (*load_shader)(struct ShaderProgram *new_prg);
    struct ShaderProgram *(*create_and_load_new_shader)(uint32_t shader_id);
    struct ShaderProgram *(*lookup_shader)(uint32_t shader_id);
    void (*shader_get_info)(struct ShaderProgram *prg, uint8_t *num_inputs, bool used_textures[2]);
    uint32_t (*new_texture)(void);
    void (*select_texture)(int tile, uint32_t texture_id);
    void (*upload_texture)(const uint8_t *rgba32_buf, int width, int height);
    void (*set_sampler_parameters)(int sampler, bool linear_filter, uint32_t cms, uint32_t cmt);
    void (*set_depth_test)(bool enable);
    void (*set_depth_compare)(uint8_t function);
    void (*set_depth_write)(bool enable);
    void (*set_polygon_offset)(bool enable);
    void (*set_viewport)(int x, int y, int width, int height);
    void (*set_scissor)(int x, int y, int width, int height);
    void (*set_blend_components)(struct WGPUBlendState component);
    void (*draw_triangles)(float buf_vbo[], size_t buf_vbo_len, size_t buf_vbo_num_tris);
    void (*init)(void);
    void (*on_resize)(void);
    void (*start_frame)(void);
    void (*end_frame)(void);
    void (*finish_render)(void);
};

struct Texture {
    uintptr_t texture_addr;
    uint8_t fmt, size;

    uint32_t texture_id;
    uint8_t cms, cmt;

    bool linear_filter;
};

struct ColorCombiner {
    uint32_t cc_id;
    struct ShaderProgram *prg;
    uint8_t shader_input_mapping[2][4];
};

struct Light_t;

struct Rect {
    uint16_t x, y, width, height;
};

struct OutputDimensions {
    uint32_t width, height;
    float aspect_ratio;
};

struct RGBA {
    uint8_t r, g, b, a;
};

struct StagingVertex {
    float x, y, z, w;
    float u, v;
    struct RGBA color;
    uint8_t clip_reject;
};

#ifdef __cplusplus
extern "C" {
#endif

void* RCPCreate();
void RCPReset(void* rcp);

// Gfx Getters and Setters
void* GfxCreateContext(struct CGraphicsDevice *rapi);
struct CGraphicsDevice* GfxGetDevice(void* gfx_context);

// F3DEX2 Commands
void F3DEX2_GSPMatrix(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPPopMatrix(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPVertex(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPMoveWord(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPMoveMem(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPTexture(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPGeometryMode(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);

void F3DEX2_GDPSetOtherModeL(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPSetOtherModeH(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPSetScissor(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPSetCombine(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPSetTile(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPLoadTile(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPSetTileSize(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPSetTextureImage(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPLoadTLUT(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);
void F3DEX2_GDPLoadBlock(void* rcp, void* gfx_context, uintptr_t w0, uintptr_t w1);

// RSP Getters and Setters
uint32_t RSPGetGeometryMode(void* rcp);
void RSPSetGeometryMode(void* rcp, uint32_t value);

struct StagingVertex* RSPGetStagingVertexAtIndexPtr(void* rcp, uint8_t index);

// RDP Getters and Setters
void RDPSetOutputDimensions(void* rcp, struct OutputDimensions dimensions);

void RDPSetViewportOrScissorChanged(void* rcp, bool value);

struct Rect RDPGetViewport(void* rcp);
struct Rect* RDPGetViewportPtr(void* rcp);
void RDPSetRenderingStateViewport(void* rcp, struct Rect viewport);
void RDPSetViewport(void* rcp, struct Rect viewport);
struct Rect* RDPGetScissorPtr(void* rcp);

void RDPSetRenderingStateScissor(void* rcp, struct Rect scissor);

void RDPFlush(void* rcp, void* gfx_context);

void RDPAddToVBOAndIncrement(void* rcp, float value);
size_t RDPIncrementTriangleCountAndReturn(void* rcp);

bool RDPLookupTexture(void* rcp, void* gfx_context, int tile, const uint8_t *orig_addr, uint32_t fmt, uint32_t size);

void RDPLookupOrCreateColorCombiner(void* rcp, void* gfx_context, uint32_t cc_id);
struct ColorCombiner* RDPGetColorCombiner(void* rcp, uint32_t cc_id);

void RDPLookupOrCreateShaderProgram(void* rcp, void* gfx_context, uint32_t shader_id);

struct ShaderProgram* RDPGetRenderingStateShaderProgram(void* rcp);
void RDPSetRenderingStateShaderProgram(void* rcp, struct ShaderProgram *prg);

struct Texture* RDPGetRenderingStateTextureAtIndex(void* rcp, int index);

bool RDPViewportDoesNotEqualRenderingStateViewport(void* rcp);
bool RDPScissorDoesNotEqualRenderingStateScissor(void* rcp);


u_int32_t RDPGetOtherModeL(void* rcp);
u_int32_t RDPGetOtherModeH(void* rcp);
void RDPSetOtherModeH(void* rcp, uint32_t value);

u_int32_t RDPGetCombineU32(void* rcp);
void* RDPGetCombine(void* rcp);
void RDPSetCombine(void* rcp, void* value);

void RDPUpdateRenderState(void* rcp, void* gfx_context, uint8_t vertex_id1, uint8_t vertex_id2, uint8_t vertex_id3);

bool RDPGetTextureChangedAtIndex(void* rcp, uint8_t index);
void RDPSetTextureChangedAtIndex(void* rcp, uint8_t index, bool value);

uint16_t RDPGetTileDescriptorTMEM(void* rcp, uint8_t index);
uint16_t RDPGetCurrentTileDescriptorULS(void* rcp);
uint16_t RDPGetCurrentTileDescriptorULT(void* rcp);
uint16_t RDPGetCurrentTileDescriptorLRS(void* rcp);
uint16_t RDPGetCurrentTileDescriptorLRT(void* rcp);
uint8_t RDPGetCurrentTileDescriptorCMS(void* rcp);
uint8_t RDPGetCurrentTileDescriptorCMT(void* rcp);
uint8_t RDPGetCurrentTileDescriptorFormat(void* rcp);
uint8_t RDPGetCurrentTileDescriptorSize(void* rcp);

uint32_t RDPGetCurrentTileDescriptorLineSizeBytes(void* rcp);

void RDPSetTMEMMap(void* rcp, uint8_t tile_number, const uint8_t* address);
uint32_t RDPGetTMEMMapEntrySize(void* rcp, uint8_t tile_number);
const uint8_t* RDPGetTMEMMapEntryAddress(void* rcp, uint8_t tile_number);

const uint8_t* RDPGetTextureImageStateAddress(void* rcp);
uint8_t RDPGetTextureImageStateSize(void* rcp);

const uint8_t* RDPPaletteAtTMEMIndex(void* rcp, uint8_t index);

void RDPImportTileTexture(void* rcp, void* gfx_context, int tile);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_RCP_H */
