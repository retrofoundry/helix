#include <libultra/ultra64.h>   // VI_*_CLOCK, AI_STATUS_*, osTvType
#include <helix/internal.h>

// libultra Audio Interface (AI) shim over the Helix-owned HLXAudio* boundary.
//
// This file holds no descriptor or occupancy model: the Arie player, source
// rate, in-flight-DMA tracker, and current-length/status accounting all live
// behind one Rust mutex (helix/src/audio.rs). The helix side is a non-gating
// tracker over a free-push submit — Arie's own software queue is the sole
// backpressure. This shim only (1) realizes the guest AI DAC frequency with the
// libultra divisor rule, mapping osTvType -> AI clock through the same TV-family
// source VI uses (HLXViGetTvFamily), and (2) applies N64 v2 argument validation
// before free-pushing a buffer across the HLXAudio boundary. Guest AI lengths
// are bytes; a stereo s16 frame is 4 bytes.

// Maximum N64 v2 AI DMA length: 18-bit length register with the bottom 3 bits
// ignored -> 0x3ffff & ~7 == 0x3fff8. Nonzero submissions must be an 8-byte
// multiple within [8, 0x3fff8]; invalid values are rejected, not rounded.
#define AI_MAX_DMA_LENGTH 0x3fff8u

// Frequency-realization rule ported from lib/src/osAiSetFrequency.c. Computes the
// DAC divisor from the AI clock and the requested rate, then reports the
// guest-realizable rate. It does not program the AI_*_REG hardware registers
// (there is no real RCP here) and does not compute the unused bitrate divisor.
// Returns 0 and writes *out_realized on success, or -1 if the requested rate is
// zero or too high for this clock (divisor < 0x84).
s32 HLXAiRealizeFrequency(u32 requested, s32 vi_clock, u32 *out_realized) {
    if (requested == 0) {
        return -1; // avoid div-by-zero; a zero rate is not realizable
    }

    float ftmp = vi_clock / (float) requested + 0.5f;
    u32 a1 = (u32) ftmp;

    if (a1 < 0x84) {
        return -1; // requested rate too high for this clock
    }

    *out_realized = (u32) ((s32) vi_clock / (s32) a1);
    return 0;
}

s32 osAiSetFrequency(u32 frequency) {
    // TV/mode -> AI clock, shared with the VI retrace clock: HLXViGetTvFamily
    // maps osTvType to NTSC=0 / PAL=1 / MPAL=2 (u32::MAX unknown).
    s32 clock;
    switch (HLXViGetTvFamily(osTvType)) {
        case 0: clock = VI_NTSC_CLOCK; break;
        case 1: clock = VI_PAL_CLOCK;  break;
        case 2: clock = VI_MPAL_CLOCK; break;
        default: return -1; // unknown TV family: no realizable clock
    }

    u32 realized;
    if (HLXAiRealizeFrequency(frequency, clock, &realized) != 0) {
        return -1;
    }

    // Installs/replaces the player for this realized source rate (a same-rate
    // call is a no-op inside the runtime). Propagate an install failure as a
    // guest -1 rather than reporting a rate the runtime never accepted.
    HLXAudioResult r = HLXAudioSetSourceRate(realized);
    if (r != HLX_AUDIO_OK) {
        return -1;
    }
    return (s32) realized;
}

s32 osAiSetNextBuffer(void *buf, u32 size) {
    // N64 v2 AI validation order: a zero-length call is a successful no-op that
    // creates no descriptor.
    if (size == 0) {
        return 0;
    }
    if (buf == NULL) {
        return -1;
    }
    if (((uintptr_t) buf) & 0x7u) {
        return -1; // addresses are 8-byte aligned
    }
    if ((size & 0x7u) != 0) {
        return -1; // lengths are 8-byte multiples (bottom 3 bits ignored)
    }
    if (size < 8u || size > AI_MAX_DMA_LENGTH) {
        return -1; // outside the modeled 18-bit DMA-length range
    }

    // Opportunistic reap, free-push to Arie, and record the DMA span only on a
    // complete receipt. No host-side capacity gate — Arie's software queue is the
    // sole backpressure. Anything but OK is a guest -1.
    return (HLXAudioSubmit(buf, size) == HLX_AUDIO_OK) ? 0 : -1;
}

u32 osAiGetLength(void) {
    // Remaining guest DMA bytes of the current in-flight DMA, straight from the
    // tracker. No host-occupancy approximation and no 32000/60 safety subtraction:
    // length comes solely from Arie's retired-source position.
    return HLXAudioCurrentLength();
}

u32 osAiGetStatus(void) {
    // Map Helix's internal AI status bits onto the guest N64 AI_STATUS_* register
    // bits (PR/rcp.h). FIFO_FULL reflects total host occupancy versus the AI
    // target depth (see audio.rs): a status-paced guest submits while
    // !AI_STATUS_FIFO_FULL, so target-depth FIFO_FULL is what lets the deep host
    // buffer fill. DMA_BUSY means an accepted DMA has not yet retired.
    u32 h = HLXAudioStatus();
    u32 out = 0;
    if (h & HLX_AI_FIFO_FULL) {
        out |= AI_STATUS_FIFO_FULL;
    }
    if (h & HLX_AI_DMA_BUSY) {
        out |= AI_STATUS_DMA_BUSY;
    }
    return out;
}
