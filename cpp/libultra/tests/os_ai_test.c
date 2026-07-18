// Standalone C test for the os_ai.c libultra AI shim.
//
// It links without the Rust helix crate: the six HLX* C entry points the shim
// calls (HLXViGetTvFamily + the five HLXAudio* boundary ops) are provided here as
// fakes that record their arguments and return scripted values. The shim's only
// other external symbol, the global `u32 osTvType` (normally defined in os_vi.c),
// is also defined here so the shim reads a value this test controls.
//
// Coverage:
//   - NTSC / PAL / MPAL divisor realization through osAiSetFrequency;
//   - a1 < 0x84 failure (requested rate too high for the clock) -> -1, no rate op;
//   - unknown TV family -> -1, no rate op;
//   - HLXAiRealizeFrequency pure boundaries: requested==0 guard, a1==0x83 fail,
//     a1==0x84 pass;
//   - osAiSetFrequency propagates a rate-install failure: a non-OK
//     HLXAudioSetSourceRate makes osAiSetFrequency return -1;
//   - osAiSetNextBuffer: zero-length no-op (no submit); null / non-8-byte address /
//     non-8-byte length / >0x3fff8 rejection (submit not called); a valid submit
//     forwards the exact (buf,size) and maps HLX_AUDIO_OK->0, QUEUE_FULL->-1;
//   - osAiGetLength returns exactly what HLXAudioCurrentLength reports;
//   - osAiGetStatus maps the helix-internal HLX_AI_* bits onto the guest
//     AI_STATUS_* register bits.

#include <ultra64.h>
#include <helix/internal.h>
#include <PR/rcp.h>

#include <stdint.h>
#include <stdio.h>

// The shim's extracted, testable pure helper (declared here so the test can hit
// the divisor boundaries directly without going through the TV-family mapping).
s32 HLXAiRealizeFrequency(u32 requested, s32 vi_clock, u32 *out_realized);

// --- Global the shim reads (normally lives in os_vi.c) ------------------------
u32 osTvType = TV_TYPE_NTSC;

// --- Fake HLX boundary: recorded calls + scripted returns --------------------
static u32            g_tv_family        = 0;   // scripted HLXViGetTvFamily result
static u32            g_tv_family_arg    = 0xDEAD;
static int            g_tv_family_calls  = 0;

static int            g_set_rate_calls   = 0;
static uint32_t       g_last_rate        = 0;
static HLXAudioResult g_set_rate_ret     = HLX_AUDIO_OK;

static int            g_submit_calls     = 0;
static const void    *g_last_submit_buf  = NULL;
static uint32_t       g_last_submit_size = 0;
static HLXAudioResult g_submit_ret       = HLX_AUDIO_OK;

static int            g_len_calls        = 0;
static uint32_t       g_len_ret          = 0;

static int            g_status_calls     = 0;
static uint32_t       g_status_ret       = 0;

static int            g_clear_calls      = 0;
static int            g_teardown_calls   = 0;

u32 HLXViGetTvFamily(u32 osTvType_arg) {
    g_tv_family_arg = osTvType_arg;
    g_tv_family_calls++;
    return g_tv_family;
}
HLXAudioResult HLXAudioSetSourceRate(uint32_t realized_rate_hz) {
    g_set_rate_calls++;
    g_last_rate = realized_rate_hz;
    return g_set_rate_ret;
}
HLXAudioResult HLXAudioSubmit(const void *stereo_i16, uint32_t byte_count) {
    g_submit_calls++;
    g_last_submit_buf  = stereo_i16;
    g_last_submit_size = byte_count;
    return g_submit_ret;
}
uint32_t HLXAudioCurrentLength(void) {
    g_len_calls++;
    return g_len_ret;
}
uint32_t HLXAudioStatus(void) {
    g_status_calls++;
    return g_status_ret;
}
HLXAudioResult HLXAudioClear(void) {
    g_clear_calls++;
    return HLX_AUDIO_OK;
}
void HLXAudioTeardown(void) {
    g_teardown_calls++;
}

// --- Test harness ------------------------------------------------------------
static int g_failures = 0;
#define CHECK(cond, msg)                                                        \
    do {                                                                        \
        if (!(cond)) {                                                          \
            printf("FAIL: %s (%s:%d)\n", (msg), __FILE__, __LINE__);            \
            g_failures++;                                                       \
        }                                                                       \
    } while (0)

static void reset_fakes(void) {
    g_tv_family = 0; g_tv_family_arg = 0xDEAD; g_tv_family_calls = 0;
    g_set_rate_calls = 0; g_last_rate = 0; g_set_rate_ret = HLX_AUDIO_OK;
    g_submit_calls = 0; g_last_submit_buf = NULL; g_last_submit_size = 0;
    g_submit_ret = HLX_AUDIO_OK;
    g_len_calls = 0; g_len_ret = 0;
    g_status_calls = 0; g_status_ret = 0;
    g_clear_calls = 0; g_teardown_calls = 0;
}

// 8-byte-aligned scratch buffer for submission tests.
static _Alignas(16) uint8_t g_buf[64];

int main(void) {
    // --- Frequency realization: family -> clock -> divisor -------------------
    // US requests 32000; NTSC clock realizes ~32006.
    reset_fakes();
    g_tv_family = 0; // NTSC
    CHECK(osAiSetFrequency(32000) == 32006, "NTSC 32000 -> 32006");
    CHECK(g_set_rate_calls == 1, "NTSC installs rate once");
    CHECK(g_last_rate == 32006, "NTSC installs realized 32006");
    CHECK(g_tv_family_arg == osTvType, "family looked up from osTvType");

    reset_fakes();
    g_tv_family = 1; // PAL
    CHECK(osAiSetFrequency(32000) == 31995, "PAL 32000 -> 31995");
    CHECK(g_last_rate == 31995 && g_set_rate_calls == 1, "PAL installs 31995");

    reset_fakes();
    g_tv_family = 2; // MPAL
    CHECK(osAiSetFrequency(32000) == 31992, "MPAL 32000 -> 31992");
    CHECK(g_last_rate == 31992 && g_set_rate_calls == 1, "MPAL installs 31992");

    // Requested rate too high for the clock: a1 < 0x84 -> -1, no rate installed.
    reset_fakes();
    g_tv_family = 0; // NTSC clock 48681812; 400000 Hz -> a1=122 < 132
    CHECK(osAiSetFrequency(400000) == -1, "too-high rate -> -1");
    CHECK(g_set_rate_calls == 0, "too-high rate installs nothing");

    // Unknown TV family -> -1, no rate installed.
    reset_fakes();
    g_tv_family = 0xFFFFFFFFu;
    CHECK(osAiSetFrequency(32000) == -1, "unknown TV family -> -1");
    CHECK(g_set_rate_calls == 0, "unknown family installs nothing");

    // Rate-install failure propagates: a realizable frequency whose backend
    // install fails makes osAiSetFrequency return -1, never the realized rate.
    reset_fakes();
    g_tv_family = 0; // NTSC realizes fine...
    g_set_rate_ret = HLX_AUDIO_BACKEND_ERROR; // ...but the install is rejected.
    CHECK(osAiSetFrequency(32000) == -1, "rate-install failure -> -1");
    CHECK(g_set_rate_calls == 1, "rate-install failure attempted one install");

    // --- Pure helper divisor boundaries (synthetic clocks) -------------------
    {
        u32 realized = 0xABCD;
        // requested==0 guard: no div-by-zero, -1.
        CHECK(HLXAiRealizeFrequency(0, VI_NTSC_CLOCK, &realized) == -1,
              "requested==0 -> -1");
        // a1 == 0x83 (131): ftmp = 131000/1000 + 0.5 = 131.5 -> a1=131 -> fail.
        CHECK(HLXAiRealizeFrequency(1000, 131000, &realized) == -1,
              "a1==0x83 -> -1");
        // a1 == 0x84 (132): ftmp = 132000/1000 + 0.5 = 132.5 -> a1=132 -> pass.
        realized = 0;
        CHECK(HLXAiRealizeFrequency(1000, 132000, &realized) == 0,
              "a1==0x84 -> 0");
        CHECK(realized == 1000, "a1==0x84 realizes 132000/132 = 1000");
        // Low extreme: freq 1 -> a1 == VI_NTSC_CLOCK (>= 0x84) -> realized 1,
        // no overflow or div-by-zero.
        realized = 0;
        CHECK(HLXAiRealizeFrequency(1, VI_NTSC_CLOCK, &realized) == 0,
              "freq==1 -> valid");
        CHECK(realized == 1, "freq==1 realizes clock/clock == 1");
        // High extreme: UINT32_MAX -> a1 rounds below 0x84 -> -1, no div-by-zero.
        CHECK(HLXAiRealizeFrequency(0xFFFFFFFFu, VI_NTSC_CLOCK, &realized) == -1,
              "freq==UINT32_MAX -> -1");
    }

    // --- osAiSetNextBuffer validation ----------------------------------------
    // Zero-length is a successful no-op: return 0, no submit / no descriptor.
    reset_fakes();
    CHECK(osAiSetNextBuffer(g_buf, 0) == 0, "zero-length -> 0");
    CHECK(g_submit_calls == 0, "zero-length does not submit");
    // Even with a NULL buffer, zero length is still the no-op.
    reset_fakes();
    CHECK(osAiSetNextBuffer(NULL, 0) == 0, "null+zero-length -> 0 no-op");
    CHECK(g_submit_calls == 0, "null+zero-length does not submit");

    // NULL buffer, nonzero length -> reject, no submit.
    reset_fakes();
    CHECK(osAiSetNextBuffer(NULL, 16) == -1, "null buf -> -1");
    CHECK(g_submit_calls == 0, "null buf does not submit");

    // Non-8-byte-aligned address -> reject, no submit. g_buf is 16-aligned;
    // g_buf+4 is 4-aligned but not 8-aligned. Length is a valid 8-multiple.
    reset_fakes();
    CHECK(osAiSetNextBuffer(g_buf + 4, 16) == -1, "misaligned addr -> -1");
    CHECK(g_submit_calls == 0, "misaligned addr does not submit");

    // Non-8-byte-multiple length -> reject, no submit.
    reset_fakes();
    CHECK(osAiSetNextBuffer(g_buf, 12) == -1, "non-8-multiple len -> -1");
    CHECK(g_submit_calls == 0, "non-8-multiple len does not submit");

    // Length > 0x3fff8 -> reject, no submit (rejected before any read).
    reset_fakes();
    CHECK(osAiSetNextBuffer(g_buf, 0x40000) == -1, ">0x3fff8 len -> -1");
    CHECK(g_submit_calls == 0, ">0x3fff8 len does not submit");

    // Valid submission: forwards exact (buf,size), maps HLX_AUDIO_OK -> 0.
    reset_fakes();
    g_submit_ret = HLX_AUDIO_OK;
    CHECK(osAiSetNextBuffer(g_buf, 16) == 0, "valid submit OK -> 0");
    CHECK(g_submit_calls == 1, "valid submit calls HLXAudioSubmit once");
    CHECK(g_last_submit_buf == (const void *) g_buf, "submit forwards buf");
    CHECK(g_last_submit_size == 16, "submit forwards size");

    // Backpressure: HLX_AUDIO_QUEUE_FULL maps to -1 (not truncated/dropped).
    reset_fakes();
    g_submit_ret = HLX_AUDIO_QUEUE_FULL;
    CHECK(osAiSetNextBuffer(g_buf, 16) == -1, "QUEUE_FULL -> -1");
    CHECK(g_submit_calls == 1, "QUEUE_FULL still attempted one submit");

    // Every non-OK result maps to -1, not just QUEUE_FULL.
    reset_fakes();
    g_submit_ret = HLX_AUDIO_BACKEND_ERROR;
    CHECK(osAiSetNextBuffer(g_buf, 16) == -1, "BACKEND_ERROR -> -1");
    CHECK(g_submit_calls == 1, "BACKEND_ERROR still attempted one submit");

    // Boundary lengths: 8 (minimum) and 0x3fff8 (maximum) are both valid and
    // forwarded verbatim. The fake HLXAudioSubmit never reads the buffer, so a
    // real 0x3fff8 span is unnecessary — only size validation/forwarding is tested.
    reset_fakes();
    g_submit_ret = HLX_AUDIO_OK;
    CHECK(osAiSetNextBuffer(g_buf, 8) == 0, "min valid len 8 -> 0");
    CHECK(g_submit_calls == 1 && g_last_submit_size == 8u, "len 8 submits size 8");
    reset_fakes();
    g_submit_ret = HLX_AUDIO_OK;
    CHECK(osAiSetNextBuffer(g_buf, 0x3fff8) == 0, "max valid len 0x3fff8 -> 0");
    CHECK(g_submit_calls == 1 && g_last_submit_size == 0x3fff8u,
          "len 0x3fff8 submits size 0x3fff8");

    // --- osAiGetLength: passthrough of the Rust descriptor length ------------
    reset_fakes();
    g_len_ret = 12345;
    CHECK(osAiGetLength() == 12345, "length reports HLXAudioCurrentLength");
    CHECK(g_len_calls == 1, "length queried once");
    reset_fakes();
    g_len_ret = 0;
    CHECK(osAiGetLength() == 0, "length 0 when Rust reports 0");

    // --- osAiGetStatus: map helix HLX_AI_* bits onto guest AI_STATUS_* bits ----
    reset_fakes();
    g_status_ret = HLX_AI_FIFO_FULL | HLX_AI_DMA_BUSY;
    CHECK(osAiGetStatus() == (AI_STATUS_FIFO_FULL | AI_STATUS_DMA_BUSY),
          "both helix bits -> both AI_STATUS bits");
    CHECK(g_status_calls == 1, "status queried once");

    reset_fakes();
    g_status_ret = HLX_AI_FIFO_FULL;
    CHECK(osAiGetStatus() == AI_STATUS_FIFO_FULL, "FIFO_FULL only -> AI_STATUS_FIFO_FULL");

    reset_fakes();
    g_status_ret = HLX_AI_DMA_BUSY;
    CHECK(osAiGetStatus() == AI_STATUS_DMA_BUSY, "DMA_BUSY only -> AI_STATUS_DMA_BUSY");

    reset_fakes();
    g_status_ret = 0;
    CHECK(osAiGetStatus() == 0, "no helix bits -> 0");

    if (g_failures == 0) {
        printf("os_ai_test: OK (all checks passed)\n");
        return 0;
    }
    printf("os_ai_test: %d FAILURE(S)\n", g_failures);
    return 1;
}
