//! Native-pointer audio runtime for the Helix libultra shim.
//!
//! Helix owns exactly one Arie 0.3 [`AudioPlayer`] — through Arie's *Rust* API,
//! never its C ABI — together with the sole N64 v2 AI descriptor tracker, behind
//! one process-global `Mutex<AudioRuntime>`. The six `HLXAudio*` `extern "C"`
//! entry points are the only boundary the C shim (`os_ai.c`) calls; each is a
//! thin wrapper that locks the one mutex and defers to a private
//! [`AudioRuntime`] method. Fixed-width scalars cross the boundary; no Arie type
//! or header does.

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use arie::{
    AudioConfig, AudioPlayer, AudioStatus, ControlError, CreateError, PlaybackState, QueueError,
    QueueReceipt, SourceChannels,
};

use model::DescriptorTracker;

pub(crate) mod model;

// C-boundary result codes — the Rust mirror of the `HLX_AUDIO_*` enum in
// `include/helix/internal.h`. `HLXAudioResult` is `i32` on both sides.
pub type HLXAudioResult = i32;
pub const HLX_AUDIO_OK: HLXAudioResult = 0;
pub const HLX_AUDIO_UNAVAILABLE: HLXAudioResult = -1;
pub const HLX_AUDIO_INVALID_ARGUMENT: HLXAudioResult = -2;
pub const HLX_AUDIO_QUEUE_FULL: HLXAudioResult = -3;
pub const HLX_AUDIO_BACKEND_ERROR: HLXAudioResult = -4;

// Helix-internal AI status bits — the Rust mirror of the `HLX_AI_*` macros in
// `include/helix/internal.h`. This is Helix's own bit layout, distinct from the
// guest N64 `AI_STATUS_*` register bits; `os_ai.c` maps these onto those.
/// Arie is `Running` and total host occupancy has reached the AI target depth.
/// See [`AudioRuntime::status`] for why both conditions: the `Running` gate keeps
/// this clear through `Prebuffering` so a status-paced guest keeps feeding to
/// Arie's startup low-water, and the target depth lets the deep host buffer fill.
pub const HLX_AI_FIFO_FULL: u32 = 0x1;
/// An accepted, unretired guest DMA exists (occupancy `> 0`) — set during
/// `Prebuffering`/`Starved` too, cleared only in terminal backend states.
pub const HLX_AI_DMA_BUSY: u32 = 0x2;

/// Target host-buffer depth Helix aims to keep queued ahead of playback, and the
/// startup prebuffer, for every Arie player it builds. The software-queue size is
/// derived per-rate from this plus one maximum legal AI DMA (see
/// [`AudioRuntime::set_source_rate`]) — no game-specific constant is involved.
const AI_TARGET_DEPTH_MS: u64 = 80;
/// Startup prebuffer duration for every Arie player Helix builds — one target
/// depth of audio.
const PREBUFFER_MS: u64 = AI_TARGET_DEPTH_MS;

/// Source frames in one maximum legal N64 v2 AI DMA: `0x3fff8` bytes / 4 bytes
/// per stereo frame. The software queue is sized to admit at least this many
/// frames on top of one target depth, so any single legal DMA always fits.
const MAX_DMA_SOURCE_FRAMES: u64 = 0x3fff8 / 4;

/// Bytes emitted per accepted stereo source frame (two interleaved `i16`).
const BYTES_PER_FRAME: u32 = 4;

/// Round `frames` at `rate` Hz up to a whole-nanosecond [`Duration`], so the
/// realized queue capacity is never a fraction of a frame short of `frames`.
fn frames_to_duration_ceil(frames: u64, rate: u32) -> Duration {
    let rate = u64::from(rate.max(1));
    let nanos = frames.saturating_mul(1_000_000_000);
    Duration::from_nanos(nanos.div_ceil(rate))
}

/// The slice of Arie's [`AudioPlayer`] the runtime depends on, abstracted so
/// tests can inject a fake without a real output device. `status` is fallible
/// here so the "status retrieval failed" boundary is representable; the real
/// player's snapshot is infallible and therefore always `Some`.
trait Player: Send {
    fn queue_interleaved_i16(&mut self, samples: &[i16]) -> Result<QueueReceipt, QueueError>;
    fn status(&self) -> Option<AudioStatus>;
    fn clear(&mut self) -> Result<(), ControlError>;
}

/// Builds a [`Player`] from an [`AudioConfig`]. Abstracted for test injection.
trait PlayerFactory: Send {
    fn create(&mut self, config: AudioConfig) -> Result<Box<dyn Player>, CreateError>;
}

impl Player for AudioPlayer {
    fn queue_interleaved_i16(&mut self, samples: &[i16]) -> Result<QueueReceipt, QueueError> {
        AudioPlayer::queue_interleaved_i16(self, samples)
    }

    fn status(&self) -> Option<AudioStatus> {
        Some(AudioPlayer::status(self))
    }

    fn clear(&mut self) -> Result<(), ControlError> {
        AudioPlayer::clear(self)
    }
}

/// Production factory: opens the default CPAL device via Arie's Rust API.
struct CpalPlayerFactory;

impl PlayerFactory for CpalPlayerFactory {
    fn create(&mut self, config: AudioConfig) -> Result<Box<dyn Player>, CreateError> {
        Ok(Box::new(AudioPlayer::new(config)?))
    }
}

/// The sole audio runtime: one optional Arie player, the active realized rate,
/// the AI descriptor tracker, and the unavailable/shutdown flags — all mutated
/// only under the process `AUDIO` mutex.
///
/// State machine (all transitions happen under the lock):
/// - fresh / setup-failed / cleared-to-lost / torn-down → `unavailable`, no
///   player: submit is [`HLX_AUDIO_UNAVAILABLE`], length is `0`;
/// - live: a player is installed at `rate` and `unavailable` is clear;
/// - `shutdown` latches after teardown; no player is ever re-created.
struct AudioRuntime {
    factory: Box<dyn PlayerFactory>,
    player: Option<Box<dyn Player>>,
    /// Realized source rate of the *installed* player. `Some` only while a
    /// player is installed; reset to `None` whenever the player is dropped, so
    /// `rate == Some(r)` always implies a live player.
    rate: Option<u32>,
    /// Non-gating tracker of in-flight guest DMA spans, used only to report the
    /// current DMA's remaining length; it never gates a submit.
    tracker: DescriptorTracker,
    /// No usable player right now. While set, submit is unavailable and length
    /// is zero.
    unavailable: bool,
    /// Latched once teardown has run; no player is re-created afterward.
    shutdown: bool,
    /// Count of guest DMAs Arie rejected with `QueueFull`. The per-rate queue
    /// sizing admits any single legal DMA, so this is unreachable in practice
    /// and should stay zero; it exists purely so a real drop is observable
    /// (telemetry) rather than silent.
    dropped_dma_count: u64,
}

impl AudioRuntime {
    fn new(factory: Box<dyn PlayerFactory>) -> Self {
        Self {
            factory,
            player: None,
            rate: None,
            tracker: DescriptorTracker::new(),
            unavailable: true,
            shutdown: false,
            dropped_dma_count: 0,
        }
    }

    /// `HLXAudioSetSourceRate`: install (or replace) the player for `rate`.
    ///
    /// Rate `0` is invalid. The same realized rate is a strict no-op. A changed
    /// rate transactionally marks unavailable, discards descriptors, drops the
    /// current player, then builds and installs a new one *only* on success.
    fn set_source_rate(&mut self, rate: u32) -> HLXAudioResult {
        if rate == 0 {
            return HLX_AUDIO_INVALID_ARGUMENT;
        }
        if self.shutdown {
            return HLX_AUDIO_UNAVAILABLE;
        }
        // Same realized rate with a live player: strict no-op, no mutation.
        if self.rate == Some(rate) && self.player.is_some() {
            return HLX_AUDIO_OK;
        }

        // Changed (or first) rate: drop the current player before building the
        // new one so at most one player ever exists.
        self.unavailable = true;
        self.rate = None;
        self.tracker.clear();
        self.player = None;

        // Size the software queue from the realized rate alone (no game-specific
        // constant): its frame capacity must admit at least one maximum legal AI
        // DMA plus one target depth of frames. This guarantees any single legal
        // guest DMA always fits; Arie's own software queue remains the sole
        // backpressure — this sizing never gates a submit.
        let target_depth_frames = u64::from(rate).saturating_mul(AI_TARGET_DEPTH_MS) / 1000;
        let capacity_frames = MAX_DMA_SOURCE_FRAMES.saturating_add(target_depth_frames);
        let config = AudioConfig {
            sample_rate_hz: rate,
            channels: SourceChannels::Stereo,
            prebuffer: Duration::from_millis(PREBUFFER_MS),
            maximum_software_queue: frames_to_duration_ceil(capacity_frames, rate),
        };
        match self.factory.create(config) {
            Ok(player) => {
                self.player = Some(player);
                self.rate = Some(rate);
                self.unavailable = false;
                HLX_AUDIO_OK
            }
            // Creation failed: stay explicitly unavailable with no player.
            Err(_) => HLX_AUDIO_BACKEND_ERROR,
        }
    }

    /// `HLXAudioSubmit`: free-push the interleaved stereo PCM straight into Arie
    /// and record the accepted DMA span; Arie's queue is the only backpressure.
    ///
    /// The descriptor tracker never gates a submit. Before pushing, an
    /// opportunistic reap at the live retired position keeps a submit-only guest
    /// (one that never polls length) from growing the tracker unbounded. A
    /// failed status snapshot neither reaps nor queues; an Arie `QueueFull` is
    /// the distinct [`HLX_AUDIO_QUEUE_FULL`].
    fn submit(&mut self, samples: &[i16], byte_count: u32) -> HLXAudioResult {
        // Defensive: the C shim collapses the zero-length no-op, but a zero or
        // frame-misaligned byte count is never a valid guest AI DMA.
        if byte_count == 0 || !byte_count.is_multiple_of(BYTES_PER_FRAME) {
            return HLX_AUDIO_INVALID_ARGUMENT;
        }
        if self.unavailable {
            return HLX_AUDIO_UNAVAILABLE;
        }
        let Some(player) = self.player.as_mut() else {
            return HLX_AUDIO_UNAVAILABLE;
        };

        // Snapshot the live retired position. A failed status snapshot neither
        // reaps nor queues: no mutation.
        let Some(status) = player.status() else {
            return HLX_AUDIO_BACKEND_ERROR;
        };
        // Opportunistic reap so a submit-only guest can't grow the tracker
        // unbounded; this never gates the push below.
        self.tracker.reap(status.retired_source_position);

        match player.queue_interleaved_i16(samples) {
            // Arie has accepted (copied) the PCM: record the DMA span so its
            // remaining length can be reported. Free-push — no capacity gate.
            Ok(receipt) => {
                self.tracker
                    .push(receipt.start_source_position, receipt.end_source_position);
                HLX_AUDIO_OK
            }
            // Arie's software queue is full — the sole, real backpressure. The
            // per-rate sizing admits any single legal DMA, so a QueueFull here
            // means a genuinely overfull host queue: count it so the drop is
            // observable rather than silent, and still report it distinctly.
            Err(QueueError::QueueFull { .. }) => {
                self.dropped_dma_count = self.dropped_dma_count.saturating_add(1);
                // Production-visible telemetry (not cfg-gated), rate-limited so a
                // pathological stall can't flood the log: the first drop and every
                // 256th are logged at warn level, so a real drop is operationally
                // observable in release, not silent.
                if self.dropped_dma_count == 1 || self.dropped_dma_count.is_multiple_of(256) {
                    log::warn!(
                        "helix audio: Arie QueueFull dropped a guest DMA (dropped_dma_count = {})",
                        self.dropped_dma_count
                    );
                }
                HLX_AUDIO_QUEUE_FULL
            }
            // Terminal Arie state (device lost / position counter exhausted): the
            // player is dead. Go unavailable so length reports 0 and later submits
            // fail cleanly, instead of retaining a dead player and a frozen,
            // never-draining current-DMA length.
            Err(QueueError::DeviceLost | QueueError::CounterExhausted) => {
                self.go_unavailable();
                HLX_AUDIO_UNAVAILABLE
            }
            Err(_) => HLX_AUDIO_BACKEND_ERROR,
        }
    }

    /// Drop the player and go explicitly unavailable, discarding tracked DMAs.
    /// Used when Arie reaches a terminal backend state; a later `set_source_rate`
    /// installs a fresh player.
    fn go_unavailable(&mut self) {
        self.player = None;
        self.rate = None;
        self.tracker.clear();
        self.unavailable = true;
    }

    /// `HLXAudioCurrentLength`: remaining guest bytes of the *current* DMA only,
    /// always bounded to a single DMA — never the summed tracker occupancy.
    ///
    /// Zero while unavailable, without a player, or when the status snapshot
    /// fails; a status failure does not reap. Otherwise report the current DMA's
    /// remaining bytes at the live retired position.
    fn current_length(&mut self) -> u32 {
        if self.unavailable {
            return 0;
        }
        // Snapshot the live retired position (and whether Arie is terminal),
        // releasing the player borrow before touching the tracker.
        let (retired, terminal) = match self.player.as_ref() {
            Some(player) => match player.status() {
                Some(status) => (
                    status.retired_source_position,
                    matches!(
                        status.state,
                        PlaybackState::DeviceLost | PlaybackState::CounterExhausted
                    ),
                ),
                // Status retrieval failed: zero without reaping.
                None => return 0,
            },
            None => return 0,
        };
        // A terminal player never drains: go unavailable so length is a clean 0
        // rather than a frozen non-zero remaining.
        if terminal {
            self.go_unavailable();
            return 0;
        }
        self.tracker.current_dma_remaining_bytes(retired)
    }

    /// `HLXAudioStatus`: the guest-visible AI status as Helix bitflags.
    ///
    /// Returns `0` when unavailable, without a player, or when the status
    /// snapshot fails. `HLX_AI_FIFO_FULL` is set only once Arie is actually
    /// `Running` and total host occupancy (`software_queued_source_frames`) has
    /// reached the AI target depth (`rate * AI_TARGET_DEPTH_MS`).
    /// `HLX_AI_DMA_BUSY` is set whenever an accepted, unretired DMA exists
    /// (occupancy `> 0`), except in the terminal backend states.
    ///
    /// FIFO_FULL uses the target depth so a status-paced guest (which submits
    /// while `!FIFO_FULL`) builds and holds Arie's deep host buffer. It also
    /// requires `Running`: while Arie is still `Prebuffering` it stays clear so
    /// the guest keeps feeding until Arie's own startup low-water (whole resampler
    /// blocks producing the device low-water) is met — a target-depth-only gate
    /// could latch full below that and deadlock the guest.
    /// DMA_BUSY mirrors the N64 register's "an accepted DMA is still in flight"
    /// meaning (true during `Prebuffering`/`Starved` when occupancy `> 0`), not
    /// "the host callback is currently playing".
    fn status(&self) -> u32 {
        if self.unavailable {
            return 0;
        }
        let Some(player) = self.player.as_ref() else {
            return 0;
        };
        let Some(s) = player.status() else {
            return 0;
        };
        let target_frames =
            u64::from(self.rate.unwrap_or(0)).saturating_mul(AI_TARGET_DEPTH_MS) / 1000;
        let mut out: u32 = 0;
        if s.state == PlaybackState::Running && s.software_queued_source_frames >= target_frames {
            out |= HLX_AI_FIFO_FULL;
        }
        let terminal = matches!(
            s.state,
            PlaybackState::DeviceLost | PlaybackState::CounterExhausted
        );
        if s.software_queued_source_frames > 0 && !terminal {
            out |= HLX_AI_DMA_BUSY;
        }
        out
    }

    /// Number of guest DMAs dropped to an Arie `QueueFull`. Test-only telemetry
    /// probe — proves a real drop is counted rather than silently swallowed.
    #[cfg(test)]
    fn dropped_dma_count(&self) -> u64 {
        self.dropped_dma_count
    }

    /// `HLXAudioClear`: clear Arie and the descriptor tracker atomically.
    ///
    /// Unavailable when no player exists. If the Arie epoch cannot be rebuilt,
    /// tracked DMAs are still discarded, the now-unusable player is dropped, and
    /// the runtime goes unavailable.
    fn clear(&mut self) -> HLXAudioResult {
        let Some(player) = self.player.as_mut() else {
            return HLX_AUDIO_UNAVAILABLE;
        };
        match player.clear() {
            Ok(()) => {
                self.tracker.clear();
                HLX_AUDIO_OK
            }
            Err(_) => {
                self.tracker.clear();
                self.player = None;
                self.rate = None;
                self.unavailable = true;
                HLX_AUDIO_BACKEND_ERROR
            }
        }
    }

    /// `HLXAudioTeardown`: detach and drop the player under the lock, discard
    /// tracked DMAs, and latch shutdown. Accepts every prior state; idempotent.
    fn teardown(&mut self) {
        self.player = None;
        self.rate = None;
        self.tracker.clear();
        self.unavailable = true;
        self.shutdown = true;
    }
}

// MARK: - Process-global runtime + C boundary

static AUDIO: OnceLock<Mutex<AudioRuntime>> = OnceLock::new();

/// The process-global audio runtime, created on first use with the real CPAL
/// factory. No device is opened until the first `HLXAudioSetSourceRate`.
fn audio() -> &'static Mutex<AudioRuntime> {
    AUDIO.get_or_init(|| Mutex::new(AudioRuntime::new(Box::new(CpalPlayerFactory))))
}

/// Lock the runtime and run `body`, recovering a poisoned lock and containing
/// any panic so none crosses the C boundary; `default` is returned on panic.
fn with_runtime<R>(default: R, body: impl FnOnce(&mut AudioRuntime) -> R) -> R {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut runtime = audio()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        body(&mut runtime)
    }));
    outcome.unwrap_or(default)
}

/// Tear the audio runtime down from Rust (the process teardown path calls this
/// directly). No-op when the runtime was never initialized.
pub(crate) fn teardown() {
    if let Some(lock) = AUDIO.get() {
        let mut runtime = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        runtime.teardown();
    }
}

/// C entry point for `HLXAudioSetSourceRate`; locks the runtime and installs or
/// replaces the player for `realized_rate_hz`.
#[no_mangle]
pub extern "C" fn HLXAudioSetSourceRate(realized_rate_hz: u32) -> HLXAudioResult {
    with_runtime(HLX_AUDIO_BACKEND_ERROR, |runtime| {
        runtime.set_source_rate(realized_rate_hz)
    })
}

/// C entry point for `HLXAudioSubmit`. `stereo_i16` must be null or address at
/// least `byte_count` readable bytes of interleaved stereo `i16`; a null pointer
/// or zero `byte_count` is rejected before any read.
#[no_mangle]
pub extern "C" fn HLXAudioSubmit(stereo_i16: *const c_void, byte_count: u32) -> HLXAudioResult {
    if stereo_i16.is_null() || byte_count == 0 {
        return HLX_AUDIO_INVALID_ARGUMENT;
    }
    // `from_raw_parts::<i16>` requires 2-byte alignment; a misaligned pointer
    // would be undefined behavior. The real caller passes an `s16*` AI buffer
    // (always aligned), but the `const void *` boundary must defend its contract.
    if !(stereo_i16 as usize).is_multiple_of(std::mem::align_of::<i16>()) {
        return HLX_AUDIO_INVALID_ARGUMENT;
    }
    // SAFETY: non-null, `i16`-aligned, and the C shim guarantees `stereo_i16`
    // addresses at least `byte_count` readable bytes. Two bytes per sample; a
    // frame-misaligned `byte_count` is rejected inside `submit` before any read.
    let samples =
        unsafe { std::slice::from_raw_parts(stereo_i16 as *const i16, (byte_count / 2) as usize) };
    with_runtime(HLX_AUDIO_BACKEND_ERROR, |runtime| {
        runtime.submit(samples, byte_count)
    })
}

/// C entry point for `HLXAudioCurrentLength`; remaining guest bytes of the
/// current DMA, or zero when unavailable.
#[no_mangle]
pub extern "C" fn HLXAudioCurrentLength() -> u32 {
    with_runtime(0, |runtime| runtime.current_length())
}

/// C entry point for `HLXAudioStatus`; the guest-visible AI status as Helix
/// bitflags (`HLX_AI_*`), or `0` when unavailable.
#[no_mangle]
pub extern "C" fn HLXAudioStatus() -> u32 {
    with_runtime(0, |runtime| runtime.status())
}

/// C entry point for `HLXAudioClear`; clears Arie and the descriptor tracker.
#[no_mangle]
pub extern "C" fn HLXAudioClear() -> HLXAudioResult {
    with_runtime(HLX_AUDIO_BACKEND_ERROR, |runtime| runtime.clear())
}

/// C entry point for `HLXAudioTeardown`; idempotently detaches and drops the
/// player under the lock. Accepts every prior state.
#[no_mangle]
pub extern "C" fn HLXAudioTeardown() {
    let _ = std::panic::catch_unwind(teardown);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use arie::{
        AudioConfig, AudioStatus, ControlError, CreateError, PlaybackState, QueueError,
        QueueReceipt,
    };

    #[test]
    fn submit_rejects_a_misaligned_pointer_before_building_a_slice() {
        // An odd (2-byte-misaligned) `const void *` must be rejected up front so
        // the `from_raw_parts::<i16>` never runs on a misaligned pointer (UB).
        // The reject short-circuits before touching the global runtime.
        let aligned = [0i16; 8]; // 2-byte aligned base
        let misaligned = (aligned.as_ptr() as *const u8).wrapping_add(1) as *const c_void;
        assert_eq!(HLXAudioSubmit(misaligned, 4), HLX_AUDIO_INVALID_ARGUMENT);
        assert_eq!(
            HLXAudioSubmit(std::ptr::null(), 4),
            HLX_AUDIO_INVALID_ARGUMENT
        );
        assert_eq!(
            HLXAudioSubmit(aligned.as_ptr() as *const c_void, 0),
            HLX_AUDIO_INVALID_ARGUMENT
        );
    }

    #[derive(Default)]
    struct FactoryState {
        create_fails: bool,
        created: usize,
        dropped: usize,
        players: Vec<Arc<Mutex<PlayerState>>>,
        /// The `AudioConfig` of the most recent `create` — lets a test assert the
        /// queue sizing the runtime derived from the realized rate.
        last_config: Option<AudioConfig>,
    }

    #[derive(Default)]
    struct PlayerState {
        retired: u64,
        queued_frames: u64,
        queue_calls: u32,
        status_fails: bool,
        next_position: u64,
        queue_error: Option<QueueError>,
        clear_fails: bool,
        clears: usize,
        /// Playback state this fake reports; `None` means the default `Running`,
        /// preserving prior behavior for tests that never set it.
        state: Option<PlaybackState>,
    }

    struct FakeFactory {
        inner: Arc<Mutex<FactoryState>>,
    }

    impl PlayerFactory for FakeFactory {
        fn create(&mut self, config: AudioConfig) -> Result<Box<dyn Player>, CreateError> {
            let mut st = self.inner.lock().unwrap();
            st.last_config = Some(config.clone());
            if st.create_fails {
                return Err(CreateError::NoOutputDevice);
            }
            let state = Arc::new(Mutex::new(PlayerState::default()));
            st.created += 1;
            st.players.push(Arc::clone(&state));
            Ok(Box::new(FakePlayer {
                state,
                factory: Arc::clone(&self.inner),
            }))
        }
    }

    struct FakePlayer {
        state: Arc<Mutex<PlayerState>>,
        factory: Arc<Mutex<FactoryState>>,
    }

    impl Player for FakePlayer {
        fn queue_interleaved_i16(&mut self, samples: &[i16]) -> Result<QueueReceipt, QueueError> {
            let mut st = self.state.lock().unwrap();
            // A configured error is returned without mutating any counter, so a
            // test can prove a rejected submit never reached Arie's queue.
            if let Some(err) = st.queue_error.clone() {
                return Err(err);
            }
            let frames = (samples.len() / 2) as u64;
            let start = st.next_position;
            let end = start + frames;
            st.next_position = end;
            st.queue_calls += 1;
            st.queued_frames += frames;
            Ok(QueueReceipt {
                start_source_position: start,
                end_source_position: end,
            })
        }

        fn status(&self) -> Option<AudioStatus> {
            let st = self.state.lock().unwrap();
            if st.status_fails {
                return None;
            }
            Some(AudioStatus {
                state: st.state.unwrap_or(PlaybackState::Running),
                software_queued_source_frames: st.queued_frames,
                software_queued_duration: Duration::ZERO,
                retired_source_position: st.retired,
                consumed_source_frames: 0,
                rejected_source_frames: 0,
                queue_full_events: 0,
                underrun_events: 0,
                underrun_device_frames: 0,
            })
        }

        fn clear(&mut self) -> Result<(), ControlError> {
            let mut st = self.state.lock().unwrap();
            if st.clear_fails {
                return Err(ControlError::DeviceLost);
            }
            st.clears += 1;
            Ok(())
        }
    }

    impl Drop for FakePlayer {
        fn drop(&mut self) {
            self.factory.lock().unwrap().dropped += 1;
        }
    }

    fn fake() -> (AudioRuntime, Arc<Mutex<FactoryState>>) {
        let inner = Arc::new(Mutex::new(FactoryState::default()));
        let runtime = AudioRuntime::new(Box::new(FakeFactory {
            inner: Arc::clone(&inner),
        }));
        (runtime, inner)
    }

    fn player(handle: &Arc<Mutex<FactoryState>>, index: usize) -> Arc<Mutex<PlayerState>> {
        Arc::clone(&handle.lock().unwrap().players[index])
    }

    fn created(handle: &Arc<Mutex<FactoryState>>) -> usize {
        handle.lock().unwrap().created
    }

    fn dropped(handle: &Arc<Mutex<FactoryState>>) -> usize {
        handle.lock().unwrap().dropped
    }

    fn set_retired(handle: &Arc<Mutex<FactoryState>>, index: usize, retired: u64) {
        player(handle, index).lock().unwrap().retired = retired;
    }

    fn set_queue_error(handle: &Arc<Mutex<FactoryState>>, index: usize, error: QueueError) {
        player(handle, index).lock().unwrap().queue_error = Some(error);
    }

    fn set_queued_frames(handle: &Arc<Mutex<FactoryState>>, index: usize, frames: u64) {
        player(handle, index).lock().unwrap().queued_frames = frames;
    }

    fn set_state(handle: &Arc<Mutex<FactoryState>>, index: usize, state: PlaybackState) {
        player(handle, index).lock().unwrap().state = Some(state);
    }

    fn last_config(handle: &Arc<Mutex<FactoryState>>) -> AudioConfig {
        handle
            .lock()
            .unwrap()
            .last_config
            .clone()
            .expect("a config was captured")
    }

    fn queue_calls(handle: &Arc<Mutex<FactoryState>>, index: usize) -> u32 {
        player(handle, index).lock().unwrap().queue_calls
    }

    #[test]
    fn setup_failure_stays_unavailable_and_installs_nothing() {
        let (mut rt, h) = fake();
        h.lock().unwrap().create_fails = true;

        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_BACKEND_ERROR);
        assert_eq!(created(&h), 0);
        assert_eq!(rt.current_length(), 0);
        // Submission while unavailable fails without touching the (absent) model.
        assert_eq!(rt.submit(&[0, 0, 0, 0], 8), HLX_AUDIO_UNAVAILABLE);
    }

    #[test]
    fn same_rate_is_a_no_op_without_mutation() {
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        // Populate one descriptor [0, 528) so we can prove no reset happened.
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);
        let before = rt.current_length();
        assert!(before > 0);

        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        // No new player, descriptors intact.
        assert_eq!(created(&h), 1);
        assert_eq!(dropped(&h), 0);
        assert_eq!(rt.current_length(), before);
    }

    #[test]
    fn changed_rate_drops_old_installs_new_and_clears_descriptors() {
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);
        assert!(rt.current_length() > 0);

        assert_eq!(rt.set_source_rate(22_050), HLX_AUDIO_OK);
        assert_eq!(created(&h), 2);
        assert_eq!(dropped(&h), 1);
        // Descriptors were cleared by the rate transition.
        assert_eq!(rt.current_length(), 0);
    }

    #[test]
    fn submit_free_pushes_every_buffer_and_never_self_wedges() {
        // Retired stays 0 here, standing in for a starve. Submit must still accept
        // every buffer -- Arie's queue is the only backpressure -- so the runtime
        // can never wedge itself when Arie stops draining.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        // `retired` stays frozen at 0 for the whole loop — the pathological case.
        let buffer = vec![0i16; 528 * 2];
        for _ in 0..64 {
            assert_eq!(rt.submit(&buffer, 528 * 4), HLX_AUDIO_OK);
        }
        // Non-vacuous: every one of the 64 submits truly reached Arie's queue,
        // even with `retired` frozen at 0.
        assert_eq!(queue_calls(&h, 0), 64);
    }

    #[test]
    fn submit_maps_arie_queue_full() {
        // Arie's `QueueFull` — the sole real backpressure — maps distinctly.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        set_queue_error(
            &h,
            0,
            QueueError::QueueFull {
                available_source_frames: 0,
            },
        );
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_QUEUE_FULL);
    }

    #[test]
    fn terminal_backend_state_goes_unavailable_gracefully() {
        // A terminal Arie state (device lost / counter exhausted) drops the dead
        // player and goes unavailable, so length is a clean 0 and later submits
        // fail cleanly — not a retained dead player with a frozen current-DMA
        // length. Composes cleanly with the free-push/faithful-length path.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);

        set_queue_error(&h, 0, QueueError::DeviceLost);
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_UNAVAILABLE);
        // Now cleanly unavailable.
        assert_eq!(rt.current_length(), 0);
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_UNAVAILABLE);
        assert_eq!(rt.status(), 0);
    }

    #[test]
    fn config_sizes_queue_for_any_legal_dma_across_the_realized_rate_range() {
        // The queue must admit at least one maximum legal AI DMA (0x3fff8 / 4 =
        // 65_534 source frames) plus one target depth at every realizable rate —
        // a queue wrong only near ~3 kHz or ~368 kHz must fail.
        // Compare durations (not round-down frames) to avoid a fencepost: Arie's
        // duration_to_frames also rounds up, so a config duration >= the ceil
        // duration for `want_frames` guarantees the realized capacity >= want.
        for rate in [3_000u32, 16_000, 22_050, 32_000, 32_006, 48_000, 368_000] {
            let (mut rt, h) = fake();
            assert_eq!(rt.set_source_rate(rate), HLX_AUDIO_OK);
            let cfg = last_config(&h);
            assert_eq!(cfg.prebuffer, Duration::from_millis(AI_TARGET_DEPTH_MS));
            let want_frames = MAX_DMA_SOURCE_FRAMES + u64::from(rate) * AI_TARGET_DEPTH_MS / 1000;
            // Independent oracle (not the production helper): the frames Arie will
            // realize from this Duration is at least `floor(ns * rate / 1e9)` (Arie
            // rounds up, so this floor is a conservative lower bound). Compute it in
            // u128 and require it to already cover one max legal DMA + one target.
            let got_frames =
                cfg.maximum_software_queue.as_nanos() * u128::from(rate) / 1_000_000_000;
            assert!(
                got_frames >= u128::from(want_frames),
                "rate {rate}: realized capacity {got_frames} frames < required {want_frames}"
            );
        }
    }

    #[test]
    fn status_fifo_full_requires_running_and_target_depth() {
        // FIFO_FULL is set only when Arie is Running and occupancy >= target. While
        // still Prebuffering it must stay clear so a status-paced guest keeps
        // feeding until Arie's own startup low-water is met — a target-depth-only
        // gate could latch full below Arie's effective startup need and deadlock
        // the guest.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        let target = 32_000u64 * AI_TARGET_DEPTH_MS / 1000; // 2560 frames

        // Prebuffering with occupancy far above target: still not full.
        set_state(&h, 0, PlaybackState::Prebuffering);
        set_queued_frames(&h, 0, target + 5000);
        assert_eq!(rt.status() & HLX_AI_FIFO_FULL, 0, "prebuffering never full");

        // Running below target: not full. At/above target: full.
        set_state(&h, 0, PlaybackState::Running);
        set_queued_frames(&h, 0, target - 1);
        assert_eq!(rt.status() & HLX_AI_FIFO_FULL, 0, "running below target");
        set_queued_frames(&h, 0, target);
        assert_eq!(
            rt.status() & HLX_AI_FIFO_FULL,
            HLX_AI_FIFO_FULL,
            "running at target"
        );
    }

    #[test]
    fn status_dma_busy_reflects_any_unretired_dma() {
        // DMA_BUSY mirrors the N64 register's "an accepted DMA is still in flight"
        // (occupancy > 0) — true during Prebuffering/Starved, not gated on the host
        // callback playing. Only terminal backend states clear it.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        set_queued_frames(&h, 0, 10);
        for s in [
            PlaybackState::Prebuffering,
            PlaybackState::Running,
            PlaybackState::Starved,
        ] {
            set_state(&h, 0, s);
            assert_eq!(
                rt.status() & HLX_AI_DMA_BUSY,
                HLX_AI_DMA_BUSY,
                "{s:?} with occupancy>0 is busy"
            );
        }
        // No unretired DMA: not busy.
        set_state(&h, 0, PlaybackState::Running);
        set_queued_frames(&h, 0, 0);
        assert_eq!(rt.status() & HLX_AI_DMA_BUSY, 0);
        // Terminal state clears busy even with stale occupancy.
        set_state(&h, 0, PlaybackState::DeviceLost);
        set_queued_frames(&h, 0, 10);
        assert_eq!(rt.status() & HLX_AI_DMA_BUSY, 0);
    }

    #[test]
    fn status_is_zero_when_unavailable_or_status_fails() {
        let (mut rt, h) = fake();
        // Fresh runtime: unavailable, no player -> zero.
        assert_eq!(rt.status(), 0);

        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        // Status snapshot failure -> zero.
        player(&h, 0).lock().unwrap().status_fails = true;
        assert_eq!(rt.status(), 0);
    }

    #[test]
    fn queue_full_submit_increments_dropped_dma_count() {
        // The per-rate sizing makes a QueueFull unreachable for any legal DMA, but
        // a real drop must be observable rather than silent: it bumps the counter
        // and still returns HLX_AUDIO_QUEUE_FULL.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt.dropped_dma_count(), 0);

        set_queue_error(
            &h,
            0,
            QueueError::QueueFull {
                available_source_frames: 0,
            },
        );
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_QUEUE_FULL);
        assert_eq!(rt.dropped_dma_count(), 1);
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_QUEUE_FULL);
        assert_eq!(rt.dropped_dma_count(), 2);

        // A non-QueueFull, non-terminal backend error is not a dropped DMA — the
        // counter is specifically QueueFull telemetry. (Terminal errors go
        // unavailable; see terminal_backend_state_goes_unavailable_gracefully.)
        set_queue_error(&h, 0, QueueError::MisalignedSamples);
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_BACKEND_ERROR);
        assert_eq!(rt.dropped_dma_count(), 2);
    }

    #[test]
    fn current_length_is_current_dma_remaining_bounded_to_one_dma() {
        // Two 528-frame DMAs: [0, 528) then [528, 1056). `current_length` must
        // report only the current DMA's remaining bytes, never the summed pair.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);

        set_retired(&h, 0, 0);
        assert_eq!(rt.current_length(), 528 * 4); // whole DMA #1 remains
        set_retired(&h, 0, 200);
        assert_eq!(rt.current_length(), (528 - 200) * 4); // DMA #1 partially drained
        set_retired(&h, 0, 528);
        assert_eq!(rt.current_length(), 528 * 4); // DMA #1 reaped, DMA #2 is current

        // Sweep retired monotonically across both DMAs on a fresh runtime: the
        // reported length is always within one DMA, never the sum of the two.
        let (mut rt2, h2) = fake();
        assert_eq!(rt2.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt2.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);
        assert_eq!(rt2.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);
        for r in 0..=(528 * 2) {
            set_retired(&h2, 0, r);
            assert!(rt2.current_length() <= 528 * 4);
        }
    }

    #[test]
    fn tracker_does_not_grow_unbounded_when_length_is_never_polled() {
        // A submit-only guest that never polls length must not accumulate spans:
        // each submit reaps opportunistically at the live retired position.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        let buffer = vec![0i16; 528 * 2];
        for i in 0..1000u64 {
            // Retired keeps pace with the DMAs already submitted, so each new
            // submit's opportunistic reap frees all prior spans first.
            set_retired(&h, 0, i * 528);
            assert_eq!(rt.submit(&buffer, 528 * 4), HLX_AUDIO_OK);
        }
        // Each iteration's opportunistic reap frees the single prior span before
        // the new push, so exactly one DMA is ever in flight — despite 1000 submits
        // and zero length polls. `== 1` (not `<= 4`) also rejects a broken tracker
        // that never pushes (would be 0) or never reaps (would be 1000), and the
        // length check confirms that one span is the just-submitted current DMA.
        assert_eq!(rt.tracker.in_flight_dmas(), 1);
        assert_eq!(rt.current_length(), 528 * 4);
    }

    #[test]
    fn current_length_reaps_with_fresh_retired_before_query() {
        // The tracker is reaped with the live retired position before the query,
        // without the caller reaping manually.
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        player(&h, 0).lock().unwrap().next_position = 100;
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK); // [100, 628)

        player(&h, 0).lock().unwrap().retired = 364;
        assert_eq!(rt.current_length(), (628 - 364) * 4); // 1056

        player(&h, 0).lock().unwrap().retired = 500;
        assert_eq!(rt.current_length(), (628 - 500) * 4); // smaller
    }

    #[test]
    fn status_failure_returns_zero_and_preserves_descriptors() {
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK); // [0, 528)

        // Status retrieval fails: length is zero and no reap occurs.
        player(&h, 0).lock().unwrap().status_fails = true;
        assert_eq!(rt.current_length(), 0);
        // Submit under status failure is a backend error and does not mutate.
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_BACKEND_ERROR);

        // Restore status with a retired position that would not reap the
        // descriptor: it is still present (proving it survived the failure).
        let p = player(&h, 0);
        {
            let mut st = p.lock().unwrap();
            st.status_fails = false;
            st.retired = 0;
        }
        assert_eq!(rt.current_length(), 2112);
    }

    #[test]
    fn clear_empties_arie_and_the_tracker_atomically() {
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);
        assert!(rt.current_length() > 0);

        assert_eq!(rt.clear(), HLX_AUDIO_OK);
        assert_eq!(rt.current_length(), 0);
        assert_eq!(player(&h, 0).lock().unwrap().clears, 1);
    }

    #[test]
    fn clear_without_a_player_is_unavailable() {
        let (mut rt, _h) = fake();
        assert_eq!(rt.clear(), HLX_AUDIO_UNAVAILABLE);
    }

    #[test]
    fn length_is_zero_while_unavailable() {
        let (mut rt, _h) = fake();
        assert_eq!(rt.current_length(), 0);
    }

    #[test]
    fn invalid_rate_zero_is_rejected() {
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(0), HLX_AUDIO_INVALID_ARGUMENT);
        assert_eq!(created(&h), 0);
    }

    #[test]
    fn teardown_is_idempotent_and_detaches_the_player() {
        let (mut rt, h) = fake();
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);
        assert_eq!(rt.submit(&[0i16; 528 * 2], 528 * 4), HLX_AUDIO_OK);

        rt.teardown();
        assert_eq!(dropped(&h), 1);
        rt.teardown(); // idempotent
        assert_eq!(dropped(&h), 1);

        // Post-teardown operations are unavailable / zero.
        assert_eq!(rt.current_length(), 0);
        assert_eq!(rt.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_UNAVAILABLE);
        assert_eq!(rt.clear(), HLX_AUDIO_UNAVAILABLE);
    }

    #[test]
    fn concurrent_submit_versus_teardown_is_safe() {
        // No ThreadSanitizer here: the toolchain is stable rustc (no
        // `-Zsanitizer`), so this exercises the mutex boundary without TSan
        // instrumentation.
        let inner = Arc::new(Mutex::new(FactoryState::default()));
        let mut rt = AudioRuntime::new(Box::new(FakeFactory {
            inner: Arc::clone(&inner),
        }));
        assert_eq!(rt.set_source_rate(32_000), HLX_AUDIO_OK);

        let runtime = Arc::new(Mutex::new(rt));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let runtime = Arc::clone(&runtime);
            handles.push(std::thread::spawn(move || {
                for _ in 0..200 {
                    let mut guard = runtime.lock().unwrap();
                    let _ = guard.submit(&[0i16; 8 * 2], 8 * 4);
                    let _ = guard.current_length();
                }
            }));
        }
        {
            let runtime = Arc::clone(&runtime);
            handles.push(std::thread::spawn(move || {
                runtime.lock().unwrap().teardown();
            }));
        }
        for handle in handles {
            handle.join().expect("worker thread joins");
        }

        // After teardown the runtime is consistently unavailable.
        let mut guard = runtime.lock().unwrap();
        assert_eq!(guard.current_length(), 0);
        assert_eq!(guard.submit(&[0i16; 8 * 2], 8 * 4), HLX_AUDIO_UNAVAILABLE);
    }
}
