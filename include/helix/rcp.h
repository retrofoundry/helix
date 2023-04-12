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

#ifdef __cplusplus
extern "C" {
#endif

void* RCPCreate(void);
void RCPReset(void* rcp);

// F3DEX2 Commands
void F3DEX2_GSPMatrix(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPMoveWord(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPMoveMem(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPTexture(void* rcp, uintptr_t w0, uintptr_t w1);
void F3DEX2_GSPGeometryMode(void* rcp, uintptr_t w0, uintptr_t w1);

// RSP Getters and Setters
uint32_t RSPGetGeometryMode(void* rcp);
void RSPSetGeometryMode(void* rcp, uint32_t value);

bool RSPGetLightsValid(void* rcp);
void RSPSetLightsValid(void* rcp, bool value);

uint8_t RSPGetNumLights(void* rcp);
void RSPSetNumLights(void* rcp, uint8_t value);

uint16_t RSPGetFogMultiplier(void* rcp);
void RSPSetFogMultiplier(void* rcp, int16_t value);
uint16_t RSPGetFogOffset(void* rcp);
void RSPSetFogOffset(void* rcp, int16_t value);

Light_t RSPGetLightAtIndex(void* rcp, uint8_t index);
Light_t* RSPGetLightAtIndexPtr(void* rcp, uint8_t index);

uint16_t RSPGetTextureScalingFactorS(void* rcp);
uint16_t RSPGetTextureScalingFactorT(void* rcp);

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
