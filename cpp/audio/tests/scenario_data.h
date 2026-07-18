#ifndef HLX_SCENARIO_DATA_H
#define HLX_SCENARIO_DATA_H
#include <ultra64.h>
#include <string.h>
// Declares fixed, deterministic scenario inputs/state. All DMEM offsets/counts
// used in scenario.inc stay within mixer.c's BUF_SIZE (0x9D0).
#define SCENARIO_DECLS                                                        \
    static const int16_t s_book[16] = {                                       \
        0x0100,0x00f0,0x00e0,0x00d0,0x00c0,0x00b0,0x00a0,0x0090,               \
        0x0080,0x0070,0x0060,0x0050,0x0040,0x0030,0x0020,0x0010 };            \
    static uint8_t s_adpcm_src[256];                                          \
    for (int _i = 0; _i < 256; _i++) s_adpcm_src[_i] = (uint8_t)(_i * 7 + 3); \
    ADPCM_STATE   s_loop_state    = {0};                                      \
    ADPCM_STATE   s_adpcm_state   = {0};                                      \
    RESAMPLE_STATE s_resample_st  = {0};                                      \
    ENVMIX_STATE  s_envmix_state  = {0};                                      \
    (void) s_loop_state; (void) s_adpcm_state;                                \
    (void) s_resample_st; (void) s_envmix_state;
#endif
