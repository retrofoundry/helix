// TODO: Remove. Bridge to test Rust implementation against working copy.

#ifndef HELIX_LIB_RCP_H
#define HELIX_LIB_RCP_H

#include <stdint.h>
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

struct ShaderProgram;

struct GfxDevice {
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
    void (*set_depth_test)(bool depth_test);
    void (*set_depth_mask)(bool z_upd);
    void (*set_zmode_decal)(bool zmode_decal);
    void (*set_viewport)(int x, int y, int width, int height);
    void (*set_scissor)(int x, int y, int width, int height);
    void (*set_use_alpha)(bool use_alpha);
    void (*draw_triangles)(float buf_vbo[], size_t buf_vbo_len, size_t buf_vbo_num_tris);
    void (*init)(void);
    void (*on_resize)(void);
    void (*start_frame)(void);
    void (*end_frame)(void);
    void (*finish_render)(void);
};

extern struct Light_t;

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

void* RCPCreate(struct GfxDevice *rapi);
void RCPReset(void* rcp);

struct GfxDevice* RCPGetGfxDevice(void* rcp);

// F3DEX2 Commands
void F3DEX2_GSPMatrix(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPPopMatrix(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPVertex(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPMoveWord(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPMoveMem(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPTexture(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPGeometryMode(void* rcp, uintptr_t w0, uintptr_t w1);

// RSP Getters and Setters
uint32_t RSPGetGeometryMode(void* rcp);
void RSPSetGeometryMode(void* rcp, uint32_t value);

struct StagingVertex* RSPGetStagingVertexAtIndexPtr(void* rcp, uint8_t index);

// RDP Getters and Setters
void RDPSetOutputDimensions(void* rcp, struct OutputDimensions dimensions);

bool RDPGetViewportOrScissorChanged(void* rcp);
void RDPSetViewportOrScissorChanged(void* rcp, bool value);

struct Rect RDPGetViewport(void* rcp);
struct Rect* RDPGetViewportPtr(void* rcp);
void RDPSetViewport(void* rcp, struct Rect viewport);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_RCP_H */
