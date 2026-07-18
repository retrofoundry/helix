#ifndef HELIX_AUDIO_MIXER_H
#define HELIX_AUDIO_MIXER_H

#include <stdbool.h>
#include <stdint.h>
#include <ultra64.h>

#ifdef VERSION_SH
#define NEW_AUDIO_UCODE
#endif

void aClearBufferImpl(uint16_t addr, int nbytes);
void aLoadADPCMImpl(int num_entries_times_16, const int16_t *book_source_addr);
void aSetBufferImpl(uint8_t flags, uint16_t in, uint16_t out, uint16_t nbytes);
void aDMEMMoveImpl(uint16_t in_addr, uint16_t out_addr, int nbytes);
void aSetLoopImpl(ADPCM_STATE *adpcm_loop_state);
void aADPCMdecImpl(uint8_t flags, ADPCM_STATE state);
void aResampleImpl(uint8_t flags, uint16_t pitch, RESAMPLE_STATE state);

#ifndef NEW_AUDIO_UCODE
void aSetVolumeImpl(uint8_t flags, int16_t v, int16_t t, int16_t r);
void aLoadBufferImpl(const void *source_addr);
void aSaveBufferImpl(int16_t *dest_addr);
void aInterleaveImpl(uint16_t left, uint16_t right);
void aMixImpl(int16_t gain, uint16_t in_addr, uint16_t out_addr);
void aEnvMixerImpl(uint8_t flags, ENVMIX_STATE state);
#else
void aLoadBufferImpl(const void *source_addr, uint16_t dest_addr, uint16_t nbytes);
void aSaveBufferImpl(uint16_t source_addr, int16_t *dest_addr, uint16_t nbytes);
void aInterleaveImpl(uint16_t dest, uint16_t left, uint16_t right, uint16_t c);
void aMixImpl(int16_t gain, uint16_t in_addr, uint16_t out_addr, uint16_t count);
void aEnvSetup1Impl(uint8_t initial_vol_wet, uint16_t rate_wet, uint16_t rate_left, uint16_t rate_right);
void aEnvSetup2Impl(uint16_t initial_vol_left, uint16_t initial_vol_right);
void aEnvMixerImpl(uint16_t in_addr, uint16_t n_samples, bool swap_reverb,
                   bool neg_left, bool neg_right,
                   uint16_t dry_left_addr, uint16_t dry_right_addr,
                   uint16_t wet_left_addr, uint16_t wet_right_addr);
void aS8DecImpl(uint8_t flags, ADPCM_STATE state);
void aAddMixerImpl(uint16_t in_addr, uint16_t out_addr, uint16_t count);
void aDuplicateImpl(uint16_t in_addr, uint16_t out_addr, uint16_t count);
void aDMEMMove2Impl(uint8_t t, uint16_t in_addr, uint16_t out_addr, uint16_t count);
void aResampleZohImpl(uint16_t pitch, uint16_t start_fract);
void aDownsampleHalfImpl(uint16_t n_samples, uint16_t in_addr, uint16_t out_addr);
void aFilterImpl(uint8_t flags, uint16_t count_or_buf, int16_t state_or_filter[8]);
void aHiLoGainImpl(uint8_t g, uint16_t count, uint16_t addr);
void aUnknown25Impl(uint8_t f, uint16_t count, uint16_t out_addr, uint16_t in_addr);
#endif

#endif
