// TODO: Remove. Bridge to test Rust implementation against working copy.

#ifndef HELIX_LIB_RCP_H
#define HELIX_LIB_RCP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

struct ShaderProgram;
struct WGPUBlendState;

typedef enum CullMode {
    CullMode_None = 0x00000000,
    CullMode_Front = 0x00000001,
    CullMode_Back = 0x00000002,
    CullMode_FrontAndBack = 0x00000003,
} CullMode;

struct CGraphicsDevice {
    bool (*z_is_from_0_to_1)(void);
    void (*unload_shader)(struct ShaderProgram *old_prg);
    void (*new_shader)(char *vertex, size_t vertex_len, char *fragment, size_t fragment_len, size_t num_floats, bool uses_tex, bool uses_tex1, bool uses_fog, bool uses_alpha, bool uses_noise, uint8_t num_inputs);
    void (*load_shader)(struct ShaderProgram *new_prg);
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
    void (*set_cull_mode)(CullMode mode);
    void (*draw_triangles)(float buf_vbo[], size_t buf_vbo_len, size_t buf_vbo_num_tris);
    void (*init)(void);
    void (*on_resize)(void);
    void (*start_frame)(void);
    void (*end_frame)(void);
    void (*finish_render)(void);
};

struct OutputDimensions {
    uint32_t width, height;
    float aspect_ratio;
};

typedef enum CCMUX {
    CCMUX_COMBINED = 0,
    CCMUX_TEXEL0 = 1,
    CCMUX_TEXEL1 = 2,
    CCMUX_PRIMITIVE = 3,
    CCMUX_SHADE = 4,
    CCMUX_ENVIRONMENT = 5,
    CCMUX_CENTER__SCALE__ONE = 6,
    // param C only
    CCMUX_COMBINED_ALPHA__NOISE__K4 = 7, // COMBINE_A only for C (ADD_ZERO?)
    CCMUX_TEXEL0_ALPHA = 8,
    CCMUX_TEXEL1_ALPHA = 9,
    CCMUX_PRIMITIVE_ALPHA = 10,
    CCMUX_SHADE_ALPHA = 11,
    CCMUX_ENV_ALPHA = 12,
    CCMUX_LOD_FRACTION = 13,
    CCMUX_PRIM_LOD_FRACTION = 14,
    CCMUX_K5 = 15, // MUL_ZERO?
    CCMUX_ZERO = 31,
} CCMUX;

typedef enum ACMUX {
    ACMUX_COMBINED__LOD_FRAC = 0, // ADD?
    ACMUX_TEXEL0 = 1,
    ACMUX_TEXEL1 = 2,
    ACMUX_PRIMITIVE = 3,
    ACMUX_SHADE = 4,
    ACMUX_ENVIRONMENT = 5,
    ACMUX_PRIM_LOD_FRAC__ONE = 6,
    ACMUX_ZERO = 7,
} ACMUX;

struct ColorCombinePass {
    CCMUX a, b, c, d;
};

struct AlphaCombinePass {
    ACMUX a, b, c, d;
};

struct CombineParams {
    struct ColorCombinePass c0;
    struct AlphaCombinePass a0;
    struct ColorCombinePass c1;
    struct AlphaCombinePass a1;
};

#ifdef __cplusplus
extern "C" {
#endif

void* RCPCreate();
void RCPReset(void* rcp);
void RCPRunDL(void* rcp, void* gfx_context, uintptr_t command);

// Gfx Getters and Setters
void* GfxCreateExternContext(struct CGraphicsDevice *rapi);
struct CGraphicsDevice* GfxGetExternDevice(void* gfx_context);

// F3DEX2 Commands
void F3DEX2_GSPMatrix(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPPopMatrix(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPVertex(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPMoveWord(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPMoveMem(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPTexture(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPGeometryMode(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPTriangle1WithIndexes(void* rcp, void* gfx_context, uint8_t vertex_index1, uint8_t vertex_index2, uint8_t vertex_index3);
void F3DEX2_GSPTriangle1(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GSPTriangle2(void* rcp, void* gfx_context, uintptr_t command);

void F3DEX2_GDPSetOtherModeL(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetOtherModeH(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetScissor(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetCombine(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetTile(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPLoadTile(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetTileSize(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetTextureImage(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPLoadTLUT(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPLoadBlock(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetEnvColor(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetPrimColor(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetFogColor(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetFillColor(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetDepthImage(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPSetColorImage(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPTextureRectangle(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2_GDPFillRectangle(void* rcp, void* gfx_context, uintptr_t command);

// F3DEX2E Commands
void F3DEX2E_GDPTextureRectangle(void* rcp, void* gfx_context, uintptr_t command);
void F3DEX2E_GDPFillRectangle(void* rcp, void* gfx_context, uintptr_t command);

// RDP Getters and Setters
void RDPSetOutputDimensions(void* rcp, struct OutputDimensions dimensions);
void RDPFlush(void* rcp, void* gfx_context);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_RCP_H */
