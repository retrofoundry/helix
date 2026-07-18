//! VI subsystem + retrace clock — the sole frame pacer.
//!
//! The clock schedules retraces from **rational absolute deadlines** measured off a
//! fixed epoch: the deadline of tick `k` is `k * 1e9 / hz` computed in `u128`, so it
//! neither drifts (no per-frame `1e9 / hz` truncation) nor bursts (a late wake posts a
//! single retrace and coalesces the counter forward instead of replaying missed ticks).
//! The TV-family -> refresh-rate mapping lives here and is exported for the audio AI
//! frequency path so VI and AI never diverge on TV timing.
use std::os::raw::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::ultra::mesg::HLXMesgSend;

/// N64 retrace rates. NTSC/MPAL pace at 60 Hz; PAL at 50 Hz.
pub const REFRESH_NTSC_HZ: u32 = 60;
pub const REFRESH_PAL_HZ: u32 = 50;

/// `osTvType` values (mirror `PR/libultra.h`). Read as the refresh fallback when no VI
/// mode index override is active.
const TV_TYPE_PAL: u32 = 0;
const TV_TYPE_NTSC: u32 = 1;
const TV_TYPE_MPAL: u32 = 2;

/// Stable family codes returned by [`HLXViGetTvFamily`] for C consumers (the audio AI
/// frequency path). NTSC and MPAL share 60 Hz but differ in DAC clock, so the family —
/// not the bare refresh rate — is the shared currency.
pub const TV_FAMILY_NTSC: u32 = 0;
pub const TV_FAMILY_PAL: u32 = 1;
pub const TV_FAMILY_MPAL: u32 = 2;
pub const TV_FAMILY_UNKNOWN: u32 = u32::MAX;

/// `ACTIVE_VI_MODE_INDEX == VI_MODE_UNSET` means "no `osViSetMode` override": resolve the
/// rate from `osTvType`. Must equal the sentinel `os_vi.c` passes for a null/out-of-range
/// mode pointer.
const VI_MODE_UNSET: u32 = u32::MAX;

/// One second in nanoseconds, in `u128` so rational deadline math never truncates before
/// the final narrowing.
const NANOS_PER_SEC: u128 = 1_000_000_000;

/// The clock waits interruptibly until `CLOCK_SPIN_MARGIN` before each absolute deadline,
/// then spin-sleeps the remainder for tight cadence. Bounds both host jitter and the
/// worst-case latency between a stop request and the next stop check.
const CLOCK_SPIN_MARGIN: Duration = Duration::from_micros(1000);

/// Injected per-wake action. Production passes `post_retrace`; unit tests pass a counter so
/// the clock machinery is exercised without touching the global VI event queue.
type TickFn = Arc<dyn Fn() + Send + Sync + 'static>;

/// Refresh families, keyed by TV type or VI mode index. Both `refresh_hz` (Rust VI) and
/// `HLXViGetTvFamily` (C audio AI) derive from this single source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TvFamily {
    Ntsc,
    Pal,
    Mpal,
}

impl TvFamily {
    /// Retrace rate for this family.
    fn hz(self) -> u32 {
        match self {
            TvFamily::Ntsc | TvFamily::Mpal => REFRESH_NTSC_HZ,
            TvFamily::Pal => REFRESH_PAL_HZ,
        }
    }

    /// Stable C-ABI code (see `TV_FAMILY_*`).
    fn code(self) -> u32 {
        match self {
            TvFamily::Ntsc => TV_FAMILY_NTSC,
            TvFamily::Pal => TV_FAMILY_PAL,
            TvFamily::Mpal => TV_FAMILY_MPAL,
        }
    }
}

/// Errors from resolving a refresh rate or scheduling a deadline.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViClockError {
    /// TV type / VI mode index does not map to a faithful refresh family (e.g. FPAL, or
    /// an unknown `osTvType`).
    UnknownMode,
    /// Retrace tick is so large its nanosecond deadline exceeds `u64` (~584 years).
    TickOverflow,
}

/// Registered VI retrace target (from osViSetEvent -> HLXViSetEvent).
/// Pointers stored as usize so the struct is Send onto the clock thread.
struct ViEvent {
    mq: usize,      // OSMesgQueue*
    msg: usize,     // OSMesg (void*)
    retrace: u32,   // post every `retrace` retraces (sm64 uses 1)
    remaining: u32, // countdown to the next post; reset to `retrace` on each fire
}

static VI_EVENT: Mutex<Option<ViEvent>> = Mutex::new(None);
/// Last framebuffer handed to osViSwapBuffer (HLE scanout no-op).
static SCANOUT_FB: AtomicUsize = AtomicUsize::new(0);
/// Active VI mode index (0..=55) chosen by `osViSetMode`; `VI_MODE_UNSET` -> fall back to
/// `osTvType`.
static ACTIVE_VI_MODE_INDEX: AtomicU32 = AtomicU32::new(VI_MODE_UNSET);
/// The process retrace clock. Guards handle creation/recreation so a rate change stops and
/// joins the old clock before spawning a fresh epoch.
static CLOCK: Mutex<Option<ClockHandle>> = Mutex::new(None);

// -------------------------------------------------------------------------------------
// Pure scheduling helpers (unit-tested)
// -------------------------------------------------------------------------------------

/// Nanosecond deadline of retrace `tick` at `hz`, measured from the clock epoch. Computed
/// in `u128` so a full hour (216_000 ticks @ 60 Hz) accrues zero truncation before the
/// final narrowing. Realistic ticks always fit `u64` (`u64::MAX` ns ≈ 584 years); use
/// [`checked_deadline_nanos`] where overflow must be handled.
pub fn deadline_nanos(tick: u64, hz: u32) -> u64 {
    (tick as u128 * NANOS_PER_SEC / hz as u128) as u64
}

/// Overflow-checked [`deadline_nanos`]: an astronomically large tick whose deadline would
/// exceed `u64` nanoseconds yields `TickOverflow` instead of wrapping.
pub fn checked_deadline_nanos(tick: u64, hz: u32) -> Result<u64, ViClockError> {
    u64::try_from(tick as u128 * NANOS_PER_SEC / hz as u128).map_err(|_| ViClockError::TickOverflow)
}

/// Smallest retrace tick whose deadline lies strictly after `now` (nanoseconds), given the
/// clock `epoch` and `hz`. A late wake coalesces to exactly ONE tick — e.g.
/// `next_tick_after(1_000_000_000, 0, 60) == 61`, never a burst of the ~60 missed ticks.
pub fn next_tick_after(now: u64, epoch: u64, hz: u32) -> u64 {
    // Smallest tick whose deadline is STRICTLY greater than `now`. Because `deadline_nanos`
    // FLOORS, a wake exactly at tick k's floored deadline must map to k + 1, not k — else a
    // boundary wake replays the same tick as a catch-up burst. Integer ceil of
    // `(elapsed + 1) * hz / 1e9`, computed in u128 and narrowed only at the end.
    let elapsed = now.saturating_sub(epoch) as u128;
    ((elapsed + 1) * hz as u128).div_ceil(NANOS_PER_SEC) as u64
}

/// Advance the tick counter after a wake: at least one past `current_tick`, but jumped
/// forward to skip any deadlines already missed while we slept. One wake => one post =>
/// one advance, regardless of how many periods elapsed.
fn advance_tick(current_tick: u64, elapsed_nanos: u64, hz: u32) -> u64 {
    (current_tick + 1).max(next_tick_after(elapsed_nanos, 0, hz))
}

/// Family for an `osTvType` value.
fn family_from_tv_type(os_tv_type: u32) -> Result<TvFamily, ViClockError> {
    match os_tv_type {
        TV_TYPE_PAL => Ok(TvFamily::Pal),
        TV_TYPE_NTSC => Ok(TvFamily::Ntsc),
        TV_TYPE_MPAL => Ok(TvFamily::Mpal),
        _ => Err(ViClockError::UnknownMode),
    }
}

/// Family for a VI mode index (`osViModeTable` layout): 0..=13 NTSC, 14..=27 PAL,
/// 28..=41 MPAL; 42..=55 (FPAL) and anything else has no faithful refresh.
fn family_from_mode(active_mode: u32) -> Result<TvFamily, ViClockError> {
    match active_mode {
        0..=13 => Ok(TvFamily::Ntsc),
        14..=27 => Ok(TvFamily::Pal),
        28..=41 => Ok(TvFamily::Mpal),
        _ => Err(ViClockError::UnknownMode),
    }
}

/// Resolve the retrace rate from the active VI mode index (preferred) or the `osTvType`
/// fallback when unset. The single centralized TV/mode -> refresh source shared with the
/// audio AI frequency path.
pub fn refresh_hz(os_tv_type: u32, active_mode: u32) -> Result<u32, ViClockError> {
    let family = if active_mode == VI_MODE_UNSET {
        family_from_tv_type(os_tv_type)?
    } else {
        family_from_mode(active_mode)?
    };
    Ok(family.hz())
}

// -------------------------------------------------------------------------------------
// The retrace clock
// -------------------------------------------------------------------------------------

/// Owned handle to a running retrace clock: an explicit stop sender plus the host thread's
/// join handle, so the clock is deterministically stoppable and joinable.
struct ClockHandle {
    hz: u32,
    stop_tx: Sender<()>,
    join: JoinHandle<()>,
}

impl ClockHandle {
    #[cfg(test)]
    fn thread_id(&self) -> std::thread::ThreadId {
        self.join.thread().id()
    }

    /// Signal the clock loop to stop and block until its host thread has joined.
    fn stop(self) {
        let _ = self.stop_tx.send(());
        let _ = self.join.join();
    }
}

/// Spawn a retrace clock at `hz`, invoking `on_tick` exactly once per retrace deadline.
/// Deadlines are absolute (off a fixed epoch), so scheduling neither drifts nor bursts.
fn spawn_clock(hz: u32, on_tick: TickFn) -> ClockHandle {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let join = thread::Builder::new()
        .name("helix-vi-retrace".into())
        // The no-op pre-post hook captures nothing, so `stop_rx` stays the thread's only
        // channel endpoint: a dropped `ClockHandle` still disconnects it and ends the loop.
        .spawn(move || clock_loop(hz, on_tick, stop_rx, || {}))
        .expect("spawn VI retrace clock");
    ClockHandle { hz, stop_tx, join }
}

/// The retrace clock loop, factored out so tests can drive it with an injected `pre_post`
/// hook. Posts exactly one retrace per absolute deadline. `pre_post` runs on this thread
/// right after the final spin-sleep and right before the post-sleep stop re-check;
/// production passes a no-op.
fn clock_loop(hz: u32, on_tick: TickFn, stop_rx: Receiver<()>, pre_post: impl Fn()) {
    let epoch = Instant::now();
    let mut tick: u64 = 1;
    // Loop ends when a tick's deadline would exceed u64 ns (~584 yr) or a stop is
    // signaled (the `break`s below).
    while let Ok(target_ns) = checked_deadline_nanos(tick, hz) {
        let target = epoch + Duration::from_nanos(target_ns);
        // Interruptible coarse wait up to the spin margin; wakes early on stop.
        let coarse = target
            .saturating_duration_since(Instant::now())
            .saturating_sub(CLOCK_SPIN_MARGIN);
        match stop_rx.recv_timeout(coarse) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        // Precise final approach to the absolute deadline.
        spin_sleep::sleep(target.saturating_duration_since(Instant::now()));
        pre_post();
        // Re-check the stop signal AFTER sleeping: a stop delivered during the coarse
        // wait's spin margin or the spin-sleep must suppress THIS tick's post. Without it a
        // stopped clock (or the old-rate clock during a rate change) emits one stale
        // retrace before joining.
        match stop_rx.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        // Exactly ONE retrace per wake — never a catch-up burst.
        on_tick();
        // Coalesce past any deadlines missed while we slept.
        let elapsed = Instant::now()
            .saturating_duration_since(epoch)
            .as_nanos()
            .min(u64::MAX as u128) as u64;
        tick = advance_tick(tick, elapsed, hz);
    }
}

/// Ensure `slot` holds a retrace clock running at `hz`. A same-rate call is a no-op; a
/// rate change stops and joins the old clock before spawning a fresh epoch. Returns true
/// when it (re)spawned. Callers hold the `CLOCK` mutex (or own `slot`).
fn apply_clock_rate(slot: &mut Option<ClockHandle>, hz: u32, on_tick: TickFn) -> bool {
    if let Some(handle) = slot.as_ref() {
        if handle.hz == hz {
            return false; // same rate: keep the running clock
        }
        slot.take().unwrap().stop(); // rate changed: stop + join before restart
    }
    *slot = Some(spawn_clock(hz, on_tick));
    true
}

/// Stop and join the process retrace clock, if running. Idempotent; the runtime teardown
/// path calls this so the pacing thread never outlives process cleanup.
pub fn stop_clock() {
    if let Some(handle) = CLOCK.lock().unwrap().take() {
        handle.stop();
    }
}

/// Resolve the active rate from the mode-index override or the C-owned `osTvType`.
/// Production-only: under `cargo test` the clock is driven with explicit rates.
#[cfg(not(test))]
fn resolve_refresh_hz() -> Result<u32, ViClockError> {
    extern "C" {
        // Process TV standard, defined in `os_vi.c`; initialized before any VI call and
        // never mutated after load.
        #[link_name = "osTvType"]
        static OS_TV_TYPE: u32;
    }
    let mode = ACTIVE_VI_MODE_INDEX.load(Ordering::Acquire);
    // SAFETY: a plain aligned `u32` read of a load-time-initialized, never-mutated global.
    let tv = unsafe { OS_TV_TYPE };
    refresh_hz(tv, mode)
}

// -------------------------------------------------------------------------------------
// Retrace posting + C ABI
// -------------------------------------------------------------------------------------

/// Post one retrace to the registered queue via the host-injector path, honoring
/// the retrace divisor. The unit test drives this directly to prove the divisor
/// (a post lands on exactly every Nth call; every call for N == 1).
fn post_retrace() {
    // Decrement the divisor countdown and (on the boundary) send, all while HOLDING the
    // VI_EVENT lock. Folding `remaining` into the event closes the M1c race: a concurrent
    // HLXViSetEvent that swaps mq/msg and reseeds the countdown can no longer interleave
    // between the countdown update and the send, so a post can never target a stale queue
    // nor spend the new event's divisor. The send is NOBLOCK and never re-enters VI, so
    // holding the lock across it is safe.
    let mut guard = VI_EVENT.lock().unwrap();
    let ev = match guard.as_mut() {
        Some(ev) => ev,
        None => return,
    };
    // Honor the retrace divisor (sm64 registers period == 1 -> every tick).
    ev.remaining = ev.remaining.saturating_sub(1);
    if ev.remaining == 0 {
        ev.remaining = ev.retrace.max(1);
        let mq = ev.mq as *mut c_void;
        let msg = ev.msg as *mut c_void;
        // NOBLOCK send == host injector: tail-insert MESG_VI_VBLANK, mark the
        // waiter (thread3) READY, wake the scheduler.
        HLXMesgSend(mq, msg, 0);
    }
}

#[no_mangle]
pub extern "C" fn HLXViSetEvent(mq: *mut c_void, msg: *mut c_void, retrace: u32) {
    let period = retrace.max(1);
    {
        let mut guard = VI_EVENT.lock().unwrap();
        // Install the event AND (re)seed its countdown under the ONE lock, so a clock post
        // can never observe a new queue paired with a stale countdown (or vice versa).
        *guard = Some(ViEvent {
            mq: mq as usize,
            msg: msg as usize,
            retrace: period,
            remaining: period,
        });
    }
    // Start (or re-pace) the sole frame clock at the rate the active VI mode / TV type
    // resolves to. Under `cargo test` the clock stays unspawned so the `post_retrace`
    // divisor test is deterministic; the loop machinery has dedicated unit tests.
    #[cfg(not(test))]
    {
        let hz = resolve_refresh_hz().unwrap_or(REFRESH_NTSC_HZ);
        let on_tick: TickFn = Arc::new(post_retrace);
        apply_clock_rate(&mut CLOCK.lock().unwrap(), hz, on_tick);
    }
}

/// Record the active VI mode index (0..=55) chosen by `osViSetMode`; `VI_MODE_UNSET`
/// clears the override so the rate falls back to `osTvType`. Re-paces a running clock only
/// when the resolved refresh rate actually changes (a same-rate mode change is a no-op).
#[no_mangle]
pub extern "C" fn HLXViSetModeIndex(index: u32) {
    ACTIVE_VI_MODE_INDEX.store(index, Ordering::Release);
    #[cfg(not(test))]
    {
        // Only re-pace an already-running clock; if `osViSetMode` precedes `osViSetEvent`
        // (the sm64 order), just record the index and let HLXViSetEvent start it.
        if let Ok(hz) = resolve_refresh_hz() {
            let mut guard = CLOCK.lock().unwrap();
            if guard.is_some() {
                let on_tick: TickFn = Arc::new(post_retrace);
                apply_clock_rate(&mut guard, hz, on_tick);
            }
        }
    }
}

/// Family code for `os_tv_type` (`TV_FAMILY_*`; `TV_FAMILY_UNKNOWN` when unmapped). Exported
/// so the audio AI frequency path resolves TV timing from the exact same mapping as VI.
#[no_mangle]
pub extern "C" fn HLXViGetTvFamily(os_tv_type: u32) -> u32 {
    family_from_tv_type(os_tv_type)
        .map(TvFamily::code)
        .unwrap_or(TV_FAMILY_UNKNOWN)
}

#[no_mangle]
pub extern "C" fn HLXViSwapBuffer(fb: *mut c_void) {
    // HLE scanout no-op: record the framebuffer; present shows the last-rendered
    // frame. Consumed by the gfx path.
    SCANOUT_FB.store(fb as usize, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ultra::mesg;
    use std::os::raw::c_void;
    use std::sync::atomic::{AtomicBool, AtomicU32};

    // Serializes tests that mutate the process-global VI_EVENT so Rust's parallel test
    // runner can't interleave them. Recovers from a poisoned lock (a panicking test) so a
    // single failure doesn't cascade into confusing poison errors.
    static VI_EVENT_TEST_LOCK: Mutex<()> = Mutex::new(());
    fn vi_event_guard() -> std::sync::MutexGuard<'static, ()> {
        VI_EVENT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ---- Rational absolute deadlines -------------------------------------------------

    #[test]
    fn rational_deadlines_do_not_accumulate_fractional_error() {
        // Tick 60 at 60 Hz is exactly one second.
        assert_eq!(deadline_nanos(60, 60), 1_000_000_000);
        // A full hour (216_000 ticks @ 60 Hz) lands on exactly 3600 s with zero
        // accumulated truncation — the whole point of the u128 rational math.
        assert_eq!(deadline_nanos(216_000, 60), 3_600_000_000_000);
    }

    #[test]
    fn checked_deadline_reports_overflow() {
        assert_eq!(checked_deadline_nanos(60, 60), Ok(1_000_000_000));
        // An astronomically large tick overflows u64 nanoseconds -> explicit error,
        // never a silently wrapped deadline.
        assert_eq!(
            checked_deadline_nanos(u64::MAX, 60),
            Err(ViClockError::TickOverflow)
        );
    }

    #[test]
    fn late_wakeup_coalesces_without_bursting() {
        // One second of elapsed time at 60 Hz means tick 60 is due; the *next* tick is
        // 61 — a single coalesced tick, never a burst of ~60 catch-up interrupts.
        assert_eq!(next_tick_after(1_000_000_000, 0, 60), 61);
    }

    #[test]
    fn next_tick_at_deadline_boundaries() {
        // Instant immediately before tick 60's deadline -> 60 is still the next tick.
        assert_eq!(next_tick_after(999_999_999, 0, 60), 60);
        // Exactly at the deadline -> advance past it to 61.
        assert_eq!(next_tick_after(1_000_000_000, 0, 60), 61);
        // Immediately after -> still 61.
        assert_eq!(next_tick_after(1_000_000_001, 0, 60), 61);
    }

    #[test]
    fn several_missed_periods_advance_by_a_single_post() {
        // Woke ~10 s late at 60 Hz: the loop posts ONCE and jumps the tick counter
        // straight to 601 rather than replaying 600 missed ticks.
        assert_eq!(advance_tick(1, 10_000_000_000, 60), 601);
        // An on-time wake advances by exactly one tick.
        assert_eq!(advance_tick(60, deadline_nanos(60, 60), 60), 61);
    }

    #[test]
    fn next_tick_is_strictly_after_floored_deadline() {
        // Regression (finding 1): at tick k's FLOORED deadline, the next scheduled tick
        // must be k + 1. The prior `floor(elapsed*hz/1e9) + 1` returned k at floored
        // deadlines — 1->1, 2->2, 10->10, 59->59, 61->61 (all wrong by one) — so a wake
        // landing exactly on a deadline replayed the same tick as a catch-up burst.
        for k in [1u64, 2, 10, 59, 61] {
            assert_eq!(
                next_tick_after(deadline_nanos(k, 60), 0, 60),
                k + 1,
                "tick {k}: next must be strictly past its floored deadline"
            );
        }
    }

    #[test]
    fn wake_at_deadline_posts_once_then_schedules_future() {
        // Loop-level (finding 1): a wake AT tick k's floored deadline advances to exactly
        // k + 1 and schedules a deadline STRICTLY in the future, so the loop sleeps rather
        // than immediately posting a second retrace. Under the old formula next_tick_after
        // returned k, whose deadline == now (not strictly after) — the burst.
        for k in [1u64, 2, 10, 59, 61] {
            let now = deadline_nanos(k, 60);
            let scheduled = next_tick_after(now, 0, 60);
            assert_eq!(scheduled, k + 1);
            // The loop's monotonic advance lands on the same tick.
            assert_eq!(advance_tick(k, now, 60), k + 1);
            assert!(
                deadline_nanos(scheduled, 60) > now,
                "tick {scheduled} deadline must be strictly after now={now}"
            );
        }
    }

    // ---- Centralized TV-family / refresh mapping -------------------------------------

    #[test]
    fn refresh_rate_per_tv_family() {
        // Unset mode -> resolve from osTvType. NTSC/MPAL run at 60 Hz, PAL at 50 Hz.
        assert_eq!(refresh_hz(TV_TYPE_NTSC, VI_MODE_UNSET), Ok(60));
        assert_eq!(refresh_hz(TV_TYPE_PAL, VI_MODE_UNSET), Ok(50));
        assert_eq!(refresh_hz(TV_TYPE_MPAL, VI_MODE_UNSET), Ok(60));
    }

    #[test]
    fn refresh_rate_unknown_tv_is_an_error() {
        assert_eq!(
            refresh_hz(99, VI_MODE_UNSET),
            Err(ViClockError::UnknownMode)
        );
    }

    #[test]
    fn active_mode_index_overrides_tv_type() {
        // A PAL mode index (16) overrides an NTSC TV type -> 50 Hz.
        assert_eq!(refresh_hz(TV_TYPE_NTSC, 16), Ok(50));
        // An NTSC mode index (2) overrides a PAL TV type -> 60 Hz.
        assert_eq!(refresh_hz(TV_TYPE_PAL, 2), Ok(60));
        // An MPAL mode index (30) -> 60 Hz.
        assert_eq!(refresh_hz(TV_TYPE_NTSC, 30), Ok(60));
        // FPAL indices (42..=55) have no faithful refresh -> error.
        assert_eq!(refresh_hz(TV_TYPE_NTSC, 42), Err(ViClockError::UnknownMode));
        // Out-of-table indices -> error.
        assert_eq!(refresh_hz(TV_TYPE_NTSC, 56), Err(ViClockError::UnknownMode));
    }

    #[test]
    fn tv_family_export_matches_refresh_mapping() {
        // HLXViGetTvFamily and refresh_hz must be the ONE shared TV source consumed by
        // both VI and the audio AI frequency path.
        assert_eq!(HLXViGetTvFamily(TV_TYPE_NTSC), TV_FAMILY_NTSC);
        assert_eq!(HLXViGetTvFamily(TV_TYPE_PAL), TV_FAMILY_PAL);
        assert_eq!(HLXViGetTvFamily(TV_TYPE_MPAL), TV_FAMILY_MPAL);
        assert_eq!(HLXViGetTvFamily(99), TV_FAMILY_UNKNOWN);
    }

    // ---- Stoppable / recreatable clock ------------------------------------------------

    fn counting_tick() -> (TickFn, Arc<AtomicU32>) {
        let ticks = Arc::new(AtomicU32::new(0));
        let sink = ticks.clone();
        let f: TickFn = Arc::new(move || {
            sink.fetch_add(1, Ordering::Relaxed);
        });
        (f, ticks)
    }

    #[test]
    fn clock_shutdown_joins_cleanly() {
        // spawn -> stop() must signal and JOIN the host thread without hanging.
        let (tick, _ticks) = counting_tick();
        let handle = spawn_clock(60, tick);
        handle.stop(); // returns only after join()
    }

    #[test]
    fn stop_before_post_suppresses_that_ticks_post() {
        // Regression (finding 2): a stop that arrives during the final spin-sleep (here
        // injected deterministically at the pre-post hook) must suppress that tick's post.
        // Without the post-sleep stop re-check the loop still calls on_tick once — one
        // stale retrace after the stop. `on_tick` here counts posts; the assertion is 0.
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let inject = stop_tx.clone();
        let fired = Arc::new(AtomicBool::new(false));
        let f = fired.clone();
        let (tick, ticks) = counting_tick();
        // 1 MHz: the first deadline is ~1 µs out, so the loop reaches the pre-post point on
        // tick 1 immediately — no real-time wait, fully deterministic.
        let join = std::thread::spawn(move || {
            clock_loop(1_000_000, tick, stop_rx, move || {
                // Inject the stop once, at the pre-post point of tick 1.
                if !f.swap(true, Ordering::Relaxed) {
                    inject.send(()).unwrap();
                }
            });
        });
        join.join().unwrap();
        drop(stop_tx);
        assert_eq!(
            ticks.load(Ordering::Relaxed),
            0,
            "a stop observed at the pre-post point must skip that tick's post"
        );
    }

    #[test]
    fn clock_recreates_only_on_rate_change() {
        let mut slot: Option<ClockHandle> = None;

        // First application spawns.
        let (t0, _c0) = counting_tick();
        assert!(apply_clock_rate(&mut slot, 60, t0));
        let id1 = slot.as_ref().unwrap().thread_id();

        // Same rate: no-op, same thread kept.
        let (t1, _c1) = counting_tick();
        assert!(!apply_clock_rate(&mut slot, 60, t1));
        assert_eq!(slot.as_ref().unwrap().thread_id(), id1);

        // Different rate: stop+join old, spawn a fresh epoch on a new thread.
        let (t2, _c2) = counting_tick();
        assert!(apply_clock_rate(&mut slot, 50, t2));
        assert_ne!(slot.as_ref().unwrap().thread_id(), id1);

        slot.take().unwrap().stop();
    }

    #[test]
    fn stop_clock_is_idempotent_when_idle() {
        // No process clock running under cfg(test): teardown hook must be a safe no-op.
        stop_clock();
        stop_clock();
    }

    // ---- Retrace divisor (unchanged, still deterministic) -----------------------------

    // Drive post_retrace() `calls` times against a queue registered with divisor
    // `n`, draining after each call; return the 1-based call numbers on which a
    // MESG_VI_VBLANK actually landed. (HLXViSetEvent does not spawn the clock under
    // cfg(test), so the only posts are the ones this loop makes — fully deterministic.)
    fn posts_on_calls(n: u32, calls: u32) -> Vec<u32> {
        let mut backing = [0u8; 64];
        let mq = backing.as_mut_ptr() as *mut c_void;
        let mut msgbuf: [*mut c_void; 4] = [std::ptr::null_mut(); 4];
        mesg::HLXMesgQueueCreate(mq, msgbuf.as_mut_ptr(), 4);
        // Register the VI target: post value 0x66, divisor n.
        HLXViSetEvent(mq, 0x66 as *mut c_void, n);

        let mut landed = Vec::new();
        for call in 1..=calls {
            post_retrace();
            // NOBLOCK recv: ret == 0 iff a post landed on this call.
            if mesg::recv(mq as usize, 0).0 == 0 {
                landed.push(call);
            }
        }
        landed
    }

    #[test]
    fn post_retrace_honors_divisor() {
        let _g = vi_event_guard();
        // N == 1: a post lands on EVERY call.
        assert_eq!(posts_on_calls(1, 5), vec![1, 2, 3, 4, 5]);
        // N == 3: a post lands on exactly every 3rd call.
        assert_eq!(posts_on_calls(3, 9), vec![3, 6, 9]);
    }

    #[test]
    fn event_replacement_routes_to_current_queue_with_fresh_divisor() {
        // Finding 3: single-threaded proof that the countdown now lives INSIDE the event,
        // so replacing the event reseeds the divisor and posts route to the CURRENT queue.
        // This exercises the decrement + replace + send-under-lock ordering. It does NOT
        // reproduce the concurrency race itself — that needed a specific interleave between
        // post_retrace's send and a concurrent HLXViSetEvent, now impossible because both
        // run under the one VI_EVENT lock.
        let _g = vi_event_guard();

        let mut a = [0u8; 64];
        let mqa = a.as_mut_ptr() as *mut c_void;
        mesg::HLXMesgQueueCreate(mqa, std::ptr::null_mut(), 4);
        HLXViSetEvent(mqa, 0xAA as *mut c_void, 3);

        // Two ticks into A's divisor of 3: countdown 3 -> 2 -> 1, nothing posted yet.
        post_retrace();
        post_retrace();
        assert_eq!(
            mesg::recv(mqa as usize, 0).0,
            -1,
            "no A post before its 3rd tick"
        );

        // Replace with queue B, divisor 2 — reseeds the countdown to B's period.
        let mut b = [0u8; 64];
        let mqb = b.as_mut_ptr() as *mut c_void;
        mesg::HLXMesgQueueCreate(mqb, std::ptr::null_mut(), 4);
        HLXViSetEvent(mqb, 0xBB as *mut c_void, 2);

        // B's divisor is honored from a FRESH countdown of 2: tick 1 silent, tick 2 posts.
        post_retrace();
        assert_eq!(
            mesg::recv(mqb as usize, 0).0,
            -1,
            "no B post before its 2nd tick"
        );
        post_retrace();
        assert_eq!(
            mesg::recv(mqb as usize, 0),
            (0, 0xBB),
            "post lands on the current queue B with B's msg"
        );

        // A never receives A's partial count nor B's post.
        assert_eq!(
            mesg::recv(mqa as usize, 0).0,
            -1,
            "stale queue A got no post"
        );
    }
}
