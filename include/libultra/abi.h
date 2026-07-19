#ifndef HELIX_LIBULTRA_ABI_H
#define HELIX_LIBULTRA_ABI_H

// Audio microcode (Acmd) ABI — helix's canonical, guest-agnostic definition.
// Words are uintptr_t (the native-width "port layer"): guest asset/audio data that embeds
// buffer pointers stores them full-width, so the acmd interpreter (helix/cpp/audio/acmd.c)
// reads them without truncation. Every faithful N64 decomp uses this same widened layout.

#include <libultra/ultratypes.h>
#include <stdint.h>

typedef struct {
    uintptr_t w0;
    uintptr_t w1;
} Awords;

typedef union {
    Awords        words;
    long long int force_structure_alignment;
} Acmd;

// F3D-era audio command opcodes (PR/abi.h).
#define A_SPNOOP     0
#define A_ADPCM      1
#define A_CLEARBUFF  2
#define A_ENVMIXER   3
#define A_LOADBUFF   4
#define A_RESAMPLE   5
#define A_SAVEBUFF   6
#define A_SEGMENT    7
#define A_SETBUFF    8
#define A_SETVOL     9
#define A_DMEMMOVE   10
#define A_LOADADPCM  11
#define A_MIXER      12
#define A_INTERLEAVE 13
#define A_POLEF      14
#define A_SETLOOP    15

// Command flag bits.
#define A_INIT     0x01
#define A_CONTINUE 0x00
#define A_LOOP     0x02
#define A_OUT      0x02
#define A_LEFT     0x02
#define A_RIGHT    0x00
#define A_VOL      0x04
#define A_RATE     0x00
#define A_AUX      0x08
#define A_NOAUX    0x00
#define A_MAIN     0x00
#define A_MIX      0x10

// Audio DMEM state arrays (saved/restored across acmd invocations).
typedef short ADPCM_STATE[16];
typedef short POLEF_STATE[4];
typedef short RESAMPLE_STATE[16];
typedef short ENVMIX_STATE[40];

#endif

