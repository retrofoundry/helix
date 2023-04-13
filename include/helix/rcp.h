// TODO: Remove. Bridge to test Rust implementation against working copy.

#ifndef HELIX_LIB_RCP_H
#define HELIX_LIB_RCP_H

#include <stdint.h>

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

void* RCPCreate(void);
void RCPReset(void* rcp);

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
