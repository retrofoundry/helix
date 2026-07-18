# Helix audio: the libultra AI adapter over Arie

This document defines the boundary between **Arie** (a general host audio library)
and **helix** (a general libultra runtime for N64 PC ports), and the contract the
helix AI adapter presents to a guest decomp (SM64 today; OOT/MM as future targets).

## Layering — who owns what

```
guest decomp (sm64 / oot / …)              libultra AI API: osAiSetFrequency,
   │  osAiSetNextBuffer / osAiGetLength /    osAiSetNextBuffer, osAiGetLength,
   │  osAiGetStatus / osAiSetFrequency       osAiGetStatus  (unchanged guest code)
   ▼
helix libultra AI shim  ── cpp/libultra/os_ai.c  +  src/audio.rs (AudioRuntime)
   │  N64 AI semantics and host feeding policy live here
   ▼  (Arie's Rust API only — never its C ABI)
Arie 0.3  ── a general push/queue host-audio library
      PCM in → resampled → played smoothly; reports occupancy + starvation.
      Contains no N64/libultra concepts. Reusable by any emulator or port.
```

**Arie stays general.** Its job is only: accept interleaved PCM at some source
rate, resample to the device, play it smoothly, and report queue depth /
starvation. An emulator for a different console reuses Arie unchanged and writes
its own thin adapter. Helix's only Arie-side change is a near-prime resampler fix
(`resample_ratio_terms`), which is itself a generality improvement — arbitrary
source rates (the N64 AI realizes near-prime rates like 32006 Hz) now resample.

**Helix owns the N64 feeding policy.** The AI's clock realization, the DMA
validation, and — crucially — the *buffering policy* that keeps Arie fed all live
in the helix shim, parameterized only by the guest's realized rate and one knob
(`AI_TARGET_DEPTH_MS`). No game-specific constant appears in `os_ai.c` / `audio.rs`.

## The C boundary — six `HLXAudio*` calls

`os_ai.c` is the only caller of the six `extern "C"` entry points in `audio.rs`;
each locks one process-global `Mutex<AudioRuntime>` (panic-contained via
`catch_unwind`) and defers to a private method. Only fixed-width scalars cross;
no Arie type or header leaks into C (`internal.h` never includes `<arie/arie.h>`).

| libultra call        | helix extern             | behavior |
|----------------------|--------------------------|----------|
| `osAiSetFrequency`   | `HLXAudioSetSourceRate`  | realize rate (divisor rule + TV family), install/replace the Arie player; propagate install failure as `-1` |
| `osAiSetNextBuffer`  | `HLXAudioSubmit`         | validate the DMA, **free-push** into Arie |
| `osAiGetLength`      | `HLXAudioCurrentLength`  | **faithful** current-DMA-remaining bytes |
| `osAiGetStatus`      | `HLXAudioStatus`         | `AI_STATUS_FIFO_FULL` / `AI_STATUS_DMA_BUSY` |
| (teardown)           | `HLXAudioClear` / `HLXAudioTeardown` | reset / drop the player |

## The feeding policy (why it is shaped this way)

The N64 AI driver paces itself off `osAiGetLength` and keeps ~1 video frame
(~18 ms) of buffer — correct for hardware, whose DAC drains sample-accurately. A
host pipeline (CPAL callback granularity + resampler, badly amplified over
Bluetooth) needs a **deep, continuously-replenished buffer (~40–80 ms)** or it
underruns. Three design choices reconcile this without lying to the guest:

1. **Free-push submit.** `osAiSetNextBuffer` pushes straight into Arie; Arie's own
   software queue is the buffer and its `QueueFull` is the *only* backpressure.
   Helix keeps **no admission FIFO** of its own. The earlier hardware-faithful
   2-deep FIFO wedged permanently on any starve: Arie's `retired` position froze,
   the FIFO filled, and no fresh audio could be admitted to un-starve it. The
   non-gating tracker (below) exists purely for reporting and never gates.
2. **Deep buffer in Arie's device ring.** `AI_TARGET_DEPTH_MS` (80 ms) sizes Arie's
   prebuffer; the ring is built during prebuffer and refilled at real time. The
   *source* queue stays shallow (~2 DMAs); the depth lives in the ring. The
   software-queue *capacity* is sized per rate to admit any single legal DMA
   (`0x3fff8` bytes) plus one target depth — a ceiling, not a latency target.
3. **Faithful `osAiGetLength`.** Reported as bytes remaining in the *current* DMA
   only (≤ one DMA), from a non-gating descriptor tracker reaped by Arie's
   `retired` position. The guest paces exactly as on hardware; the host depth is
   supplied by the prebuffer, not by lying about the queue length.

## Two host-functional deviations

Where strict hardware fidelity would break host playback, helix chooses
host-functional behavior and documents it:

- **`osAiGetLength` is conservative, ~one resampler block late.** Arie retires a
  source block only once the *next* block's output has played (the retire
  watermark), so the reported "current DMA remaining" lags true playback by up to
  one input chunk. Bounded by the resampler chunk; absorbed by the prebuffer.
- **`osAiGetStatus`'s `FIFO_FULL` is running-and-target-depth, not a 2-deep FIFO.**
  It is set only once Arie is actually `Running` **and** total host occupancy ≥
  `AI_TARGET_DEPTH_MS`. Two reasons: (a) target-depth rather than a shallow 2-deep
  gate, so a status-paced guest builds and holds the deep host buffer instead of
  starving Arie; (b) the `Running` requirement — while Arie is still
  `Prebuffering`, `FIFO_FULL` stays clear so the guest keeps feeding until Arie's
  own startup low-water (whole resampler blocks producing the device low-water) is
  met. A target-depth-*only* gate would latch full below Arie's effective startup
  need and deadlock a status-paced guest.
- **`osAiGetStatus`'s `DMA_BUSY` = an accepted, unretired DMA exists.** It is set
  whenever occupancy `> 0` — true during `Prebuffering` and `Starved`, not gated on
  the host callback playing — matching the N64 register's "a DMA is still in
  flight" meaning; only the terminal backend states (`DeviceLost`,
  `CounterExhausted`) clear it.

## OOT / MM readiness

The adapter is **game-agnostic**: it implements the standard libultra AI calls with
no SM64-specific values, so any decomp that drives audio through them should work
unchanged. Two caveats, validated only against SM64 (length-paced) so far:

- The `osAiGetStatus` `FIFO_FULL` = target-depth choice is validated against a
  length-paced guest (SM64 uses `osAiGetLength`, not status). A guest that paces
  submission purely on `AI_STATUS_FIFO_FULL` is handled by design but unverified.
- When OOT/MM sources are pinned in-tree, inspect every `osAiGetLength` /
  `osAiGetStatus` call site and add a per-driver trace-compatibility test.

### Known limitation (outside the validated SM64 profile)

The software queue is sized to admit any single legal AI DMA *in frames*, but Arie
independently caps the number of *queued source chunks*
(`MAXIMUM_SOURCE_CHUNKS = 4096`). Those 4096 chunks hold only 8192 source frames
when each DMA is the minimum legal 2 frames — **below** the per-rate frame
capacity at *every* supported rate, not just at the high end. So a guest that
submits thousands of very small (sub-frame-target) DMAs faster than Arie's worker
drains them into chunks could hit the chunk cap and see a spurious `QueueFull`
before the frame capacity is reached.

This is **not** exercised by SM64, which submits video-frame-sized DMAs (hundreds
of frames) at ~16–48 kHz — so the frame-capacity guarantee holds for the validated
SM64 profile. It has **not** been audited for OOT/MM. If a future guest submits
many tiny DMAs, eliminate the bound by coalescing adjacent submissions in the shim
or raising Arie's chunk cap; until then this is a documented limitation of the
"general" claim, not a proven-universal guarantee.

## Validation

Correctness of the tracker, free-push, faithful length, `osAiGetStatus`, and
queue sizing is unit-tested (helix `cargo test --lib audio`, ctest `helix_os_ai`).
A deterministic-ish sim of the real Arie pipeline (`arie` `pipeline.rs`,
`run_faithful_sim`) is a **rough sanity check, not proof** — it is non-deterministic
(worker thread + timing) and single-phase. The **authoritative gate is the in-game
listen test on real Bluetooth hardware**, where audio must start and *sustain*
across intro / file-select / in-level with no deadlock or dropout.
