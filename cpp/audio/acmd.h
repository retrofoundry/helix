#ifndef HELIX_AUDIO_ACMD_H
#define HELIX_AUDIO_ACMD_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
// Walk `num_cmds` 16-byte Acmd entries; dispatch each to the mixer kernels.
void hlx_acmd_process(const void *list, int32_t num_cmds);
#ifdef __cplusplus
}
#endif
#endif
