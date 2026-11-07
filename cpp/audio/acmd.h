#ifndef REALITY64_AUDIO_ACMD_H
#define REALITY64_AUDIO_ACMD_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
// Walk `num_cmds` 16-byte Acmd entries; dispatch each to the mixer kernels.
void r64_acmd_process(const void *list, int32_t num_cmds);
#ifdef __cplusplus
}
#endif
#endif
