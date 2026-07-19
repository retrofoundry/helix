use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Condvar, Mutex, OnceLock};

#[derive(Clone, Copy, Debug)]
pub struct SchedEntry {
    pub pri: i32,
    pub ready: bool,
}

/// Choose the next thread to hold the run token: the highest-priority READY thread with
/// priority > 0. Returns None when the only READY threads are pri-0 — the scheduler then
/// idles (park-on-pri-0). On a priority tie the currently-running thread keeps
/// the token (cooperative: equal priority never preempts).
pub fn pick_next(entries: &HashMap<usize, SchedEntry>, current: Option<usize>) -> Option<usize> {
    let mut best: Option<(i32, usize)> = None;
    for (&id, entry) in entries.iter() {
        if entry.ready && entry.pri > 0 {
            match best {
                Some((bp, bid)) if bp > entry.pri || (bp == entry.pri && bid <= id) => {}
                _ => best = Some((entry.pri, id)),
            }
        }
    }
    let (best_pri, best_id) = best?;
    if let Some(cur) = current {
        if let Some(entry) = entries.get(&cur) {
            if entry.ready && entry.pri > 0 && entry.pri == best_pri {
                return Some(cur);
            }
        }
    }
    Some(best_id)
}

/// osYieldThread variant of `pick_next`: round-robin among the highest-priority READY threads
/// so an equal-priority peer runs instead of being starved (unlike `pick_next`, which keeps the
/// current thread on a tie). Returns the caller if it is the sole thread at the top priority, or
/// None if no pri>0 thread is READY.
pub fn pick_next_yield(entries: &HashMap<usize, SchedEntry>, caller: usize) -> Option<usize> {
    let best_pri = entries
        .values()
        .filter(|e| e.ready && e.pri > 0)
        .map(|e| e.pri)
        .max()?;
    let mut cands: Vec<usize> = entries
        .iter()
        .filter(|(_, e)| e.ready && e.pri == best_pri)
        .map(|(&id, _)| id)
        .collect();
    cands.sort_unstable();
    match cands.iter().position(|&id| id == caller) {
        // caller is at the top priority: rotate to the next candidate (round-robin among ties).
        Some(pos) => Some(cands[(pos + 1) % cands.len()]),
        // caller is below the top priority: run the highest-priority READY thread.
        None => Some(cands[0]),
    }
}

/// Timeout a sole-runnable `reschedule_yield` waits on the condvar before re-polling. A waker's
/// `notify_all` releases it sooner; this only caps the no-event case.
#[cfg_attr(test, allow(dead_code))] // used by the non-test `yield_wait_timeout` below
const YIELD_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(2);

/// Effective sole-wait timeout. Tests can raise it so a broken wake path can't be masked by
/// timeout polling (with a large timeout only an actual `wake` releases the waiter).
#[cfg(not(test))]
fn yield_wait_timeout() -> std::time::Duration {
    YIELD_WAIT_TIMEOUT
}
#[cfg(test)]
pub(crate) static TEST_YIELD_TIMEOUT_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(2);
#[cfg(test)]
fn yield_wait_timeout() -> std::time::Duration {
    std::time::Duration::from_millis(TEST_YIELD_TIMEOUT_MS.load(std::sync::atomic::Ordering::SeqCst))
}
/// Test-only: times a thread has entered the sole-runnable wait, so a test can wait until the
/// caller is provably parked before injecting a wake.
#[cfg(test)]
pub(crate) static TEST_SOLE_WAITS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
/// Test-only: keys of threads that have completed `on_exit`, for deterministic teardown.
#[cfg(test)]
pub(crate) static TEST_EXITED: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

struct SchedInner {
    entries: HashMap<usize, SchedEntry>,
    running: Option<usize>,
    /// Bumped by `wake` whenever a thread is marked READY. A sole-runnable `reschedule_yield`
    /// waits on the condvar for this to change; read/written under the Mutex, so the wakeup can't
    /// be lost.
    wake_gen: u64,
}

/// Process-global cooperative-priority scheduler: a single run token behind a Mutex+Condvar.
/// Only the token holder runs guest code. reschedule() is called at scheduling points
/// (osStartThread, osSetThreadPri; the blocking primitives below feed the joins).
pub struct Scheduler {
    inner: Mutex<SchedInner>,
    cv: Condvar,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            inner: Mutex::new(SchedInner {
                entries: HashMap::new(),
                running: None,
                wake_gen: 0,
            }),
            cv: Condvar::new(),
        }
    }

    /// Register a created-but-not-started thread (osCreateThread): known, not yet runnable.
    pub fn register(&self, key: usize, pri: i32) {
        let mut g = self.inner.lock().unwrap();
        g.entries.insert(key, SchedEntry { pri, ready: false });
    }

    /// Mark a thread runnable (osStartThread).
    pub fn mark_ready(&self, key: usize) {
        let mut g = self.inner.lock().unwrap();
        if let Some(entry) = g.entries.get_mut(&key) {
            entry.ready = true;
        }
    }

    /// Change a thread's priority (osSetThreadPri). Does not reschedule by itself.
    pub fn set_pri(&self, key: usize, pri: i32) {
        let mut g = self.inner.lock().unwrap();
        if let Some(entry) = g.entries.get_mut(&key) {
            entry.pri = pri;
        }
    }

    /// Block the calling host thread until it holds the run token.
    pub fn acquire(&self, key: usize) {
        let mut g = self.inner.lock().unwrap();
        while g.running != Some(key) {
            g = self.cv.wait(g).unwrap();
        }
    }

    /// A scheduling point. Recompute the token holder; if `caller` no longer holds it,
    /// block `caller` until it does again (a pri-0 self-park blocks here indefinitely).
    /// `caller == None` means the process main thread (never a token holder): hand the
    /// token off and return without blocking.
    pub fn reschedule(&self, caller: Option<usize>) {
        let mut g = self.inner.lock().unwrap();
        g.running = pick_next(&g.entries, g.running);
        self.cv.notify_all();
        if let Some(me) = caller {
            while g.running != Some(me) {
                g = self.cv.wait(g).unwrap();
            }
        }
    }

    /// osYieldThread: hand the token to the next equal-or-higher READY thread (round-robin among
    /// equals). If the caller is the sole top choice it keeps the token but waits on the condvar
    /// for an injected event (`wake`) or a bounded timeout rather than spinning, then re-checks —
    /// handing off to any equal-or-higher thread that became READY before returning. `running` is
    /// always `Some(me)` on return (the caller never runs guest code without the token).
    ///
    /// Event-coupled, not a hard guarantee: it relies on the host eventually scheduling the
    /// tokenless waker. Like real osYieldThread it does not yield to a strictly-lower-priority
    /// thread, so a dependency on such a producer needs a real blocking op instead.
    pub fn reschedule_yield(&self, caller: Option<usize>) {
        let me = match caller {
            // A tokenless (non-guest) caller holds no token; just relinquish the host cpu.
            None => {
                std::thread::yield_now();
                return;
            }
            Some(m) => m,
        };
        let mut g = self.inner.lock().unwrap();
        // At most one blocking wait, then loop back once to recompute the handoff.
        let mut waited = false;
        loop {
            match pick_next_yield(&g.entries, me) {
                Some(next) if next != me => {
                    // Equal-or-higher peer READY: hand off and wait until we hold the token again.
                    g.running = Some(next);
                    self.cv.notify_all();
                    while g.running != Some(me) {
                        g = self.cv.wait(g).unwrap();
                    }
                    return;
                }
                // Still the sole choice after our one wait: resume, holding the token.
                _ if waited => return,
                _ => {
                    // Sole choice: wait for a `wake` (wake_gen bump), the token being taken, or the
                    // timeout. Reading wake_gen under the lock we wait on avoids a lost wakeup.
                    #[cfg(test)]
                    TEST_SOLE_WAITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let start = g.wake_gen;
                    let (guard, _timed_out) = self
                        .cv
                        .wait_timeout_while(g, yield_wait_timeout(), |inner| {
                            inner.wake_gen == start && inner.running == Some(me)
                        })
                        .unwrap();
                    g = guard;
                    // A tokenless `reschedule` can move `running` off `me` without bumping wake_gen;
                    // never return without the token — reacquire if it was taken (strict-priority
                    // suspension: a strictly-higher thread may legitimately keep it).
                    while g.running != Some(me) {
                        g = self.cv.wait(g).unwrap();
                    }
                    waited = true;
                    // Loop once more to recompute the handoff, then return.
                }
            }
        }
    }

    /// A thread's entry function returned: retire it and pass the token on.
    pub fn on_exit(&self, key: usize) {
        let mut g = self.inner.lock().unwrap();
        if let Some(entry) = g.entries.get_mut(&key) {
            entry.ready = false;
        }
        g.running = pick_next(&g.entries, None);
        self.cv.notify_all();
        #[cfg(test)]
        TEST_EXITED.lock().unwrap().push(key);
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

static SCHEDULER: OnceLock<Scheduler> = OnceLock::new();

/// The process-global scheduler.
pub fn scheduler() -> &'static Scheduler {
    SCHEDULER.get_or_init(Scheduler::new)
}

// --- Blocking layer (consumed by the mesg/event modules) -------------------
//
// Two-state model: `running: Option<usize>` + per-thread `{pri, ready}`. The invariant is
// `running == Some(x) ⟺ x is executing ⟺ a waker DEFERS`; `running == None ⟺ dispatch now`.
// A blocked thread never stays `running` because `park_self` hands the token off BEFORE it
// waits, and `park_self` re-runs `pick_next` after `set_blocked` — closing the lost-wakeup
// window (a racing `wake` that set `ready` is seen by that pick_next).

thread_local! {
    // The OSThread* key this host thread runs guest code for; None on the process main /
    // injector threads. Set by the thread.rs entry trampoline. Moved here from thread.rs
    // so the blocking layer can read the caller's identity.
    static CURRENT_ID: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Record that the calling host thread now runs guest code for OSThread `id`. Called by the
/// thread.rs entry trampoline before it acquires the token.
pub fn set_current_id(id: usize) {
    CURRENT_ID.with(|c| c.set(Some(id)));
}

/// The OSThread* key of the calling thread, or 0 on host/injector threads.
pub fn current_id() -> usize {
    CURRENT_ID.with(|c| c.get().unwrap_or(0))
}

/// Mark a thread blocked (osRecvMesg with an empty queue). Under the sched lock; does NOT
/// hand off the token — the token-holder follows with `park_self()`.
pub fn set_blocked(id: usize) {
    let mut g = scheduler().inner.lock().unwrap();
    if let Some(e) = g.entries.get_mut(&id) {
        e.ready = false;
    }
}

/// Wake a blocked thread (osSendMesg / event post). Callable from ANY thread, including
/// tokenless host threads (the VI clock). Recomputes `running` ONLY when the token is
/// unheld (`running == None`); when a holder is present the woken thread is left READY and
/// dispatched at the holder's next scheduling point (deferred handoff — never mid-execution).
pub fn wake(id: usize) {
    let mut g = scheduler().inner.lock().unwrap();
    if let Some(e) = g.entries.get_mut(&id) {
        e.ready = true;
    }
    // Release any sole-runnable reschedule_yield waiter (event injected).
    g.wake_gen = g.wake_gen.wrapping_add(1);
    if g.running.is_none() {
        g.running = pick_next(&g.entries, None);
    }
    scheduler().cv.notify_all();
}

/// The token-holder blocks itself: hand the token to the next runnable thread (ALWAYS
/// recompute `pick_next` first — `me` is already `ready == false` via `set_blocked`), then
/// wait until this thread holds the token again.
pub fn park_self() {
    let me = current_id();
    let mut g = scheduler().inner.lock().unwrap();
    g.running = pick_next(&g.entries, g.running);
    scheduler().cv.notify_all();
    while g.running != Some(me) {
        g = scheduler().cv.wait(g).unwrap();
    }
}

/// Test-only introspection: is thread `id` registered and currently NOT ready?
/// A guest that has parked in a blocking primitive has run `set_blocked` (ready=false)
/// followed by `park_self`, so once it is genuinely blocked this returns true. Lets a
/// test deterministically confirm "the waiter actually parked" instead of guessing with
/// a fixed sleep. Gated `#[cfg(test)]` so it never enters a production build.
#[cfg(test)]
pub(crate) fn is_blocked(id: usize) -> bool {
    let g = scheduler().inner.lock().unwrap();
    g.entries.get(&id).map(|e| !e.ready).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn e(pri: i32) -> SchedEntry {
        SchedEntry { pri, ready: true }
    }

    #[test]
    fn park_on_pri_zero_selects_nothing() {
        // A lone READY pri-0 thread (idle after osSetThreadPri(NULL,0)) is NEVER dispatched.
        let mut m = HashMap::new();
        m.insert(1usize, e(0));
        assert_eq!(pick_next(&m, Some(1)), None);
    }

    #[test]
    fn equal_priority_keeps_running_thread() {
        // idle(100,running) starts thread3(100,ready): a tie must NOT preempt the caller.
        let mut m = HashMap::new();
        m.insert(1usize, e(100)); // idle (currently running)
        m.insert(3usize, e(100)); // thread3 just started
        assert_eq!(pick_next(&m, Some(1)), Some(1));
    }

    #[test]
    fn idle_parks_then_thread3_runs() {
        let mut m = HashMap::new();
        m.insert(1usize, e(100));
        m.insert(3usize, e(100));
        m.get_mut(&1).unwrap().pri = 0; // idle -> osSetThreadPri(NULL,0)
        assert_eq!(pick_next(&m, Some(1)), Some(3));
        m.get_mut(&3).unwrap().pri = 0; // now only pri-0 threads ready -> park
        assert_eq!(pick_next(&m, Some(3)), None);
    }

    #[test]
    fn strictly_higher_priority_preempts() {
        let mut m = HashMap::new();
        m.insert(5usize, e(10)); // loop thread running
        m.insert(3usize, e(100)); // thread3 became ready (higher)
        assert_eq!(pick_next(&m, Some(5)), Some(3));
    }

    #[test]
    fn yield_hands_off_to_strictly_higher() {
        // thread5 (pri 10) yields while thread3 (pri 100) is READY -> thread3 runs.
        let mut m = HashMap::new();
        m.insert(5usize, e(10));
        m.insert(3usize, e(100));
        assert_eq!(pick_next_yield(&m, 5), Some(3));
    }

    #[test]
    fn yield_keeps_sole_top_thread() {
        // Nothing else at/above the caller's priority: the yield keeps the caller.
        let mut m = HashMap::new();
        m.insert(5usize, e(10));
        m.insert(3usize, SchedEntry { pri: 100, ready: false }); // higher but blocked
        assert_eq!(pick_next_yield(&m, 5), Some(5));
    }

    #[test]
    fn yield_round_robins_among_equal_priority() {
        // Three equal-priority READY peers: a yield rotates to the next id, then wraps —
        // unlike pick_next, which would keep the current holder forever (starvation).
        let mut m = HashMap::new();
        m.insert(5usize, e(10));
        m.insert(6usize, e(10));
        m.insert(7usize, e(10));
        assert_eq!(pick_next_yield(&m, 5), Some(6));
        assert_eq!(pick_next_yield(&m, 6), Some(7));
        assert_eq!(pick_next_yield(&m, 7), Some(5)); // wrap
        // Contrast: the non-yield pick keeps the running thread on a tie.
        assert_eq!(pick_next(&m, Some(5)), Some(5));
    }

    #[test]
    fn picks_highest_priority_ready_thread() {
        let mut entries: HashMap<usize, SchedEntry> = HashMap::new();
        entries.insert(
            0,
            SchedEntry {
                pri: 10,
                ready: true,
            },
        );
        entries.insert(
            1,
            SchedEntry {
                pri: 100,
                ready: true,
            },
        );
        entries.insert(
            2,
            SchedEntry {
                pri: 50,
                ready: true,
            },
        );
        assert_eq!(pick_next(&entries, None), Some(1));
    }

    #[test]
    fn never_dispatches_pri_0() {
        let mut entries: HashMap<usize, SchedEntry> = HashMap::new();
        entries.insert(
            0,
            SchedEntry {
                pri: 0,
                ready: true,
            },
        );
        entries.insert(
            1,
            SchedEntry {
                pri: 127,
                ready: false,
            },
        );
        assert_eq!(pick_next(&entries, None), None);
        entries.insert(
            1,
            SchedEntry {
                pri: 100,
                ready: true,
            },
        );
        assert_eq!(pick_next(&entries, None), Some(1));
    }

    #[test]
    fn no_ready_thread_parks() {
        let mut entries: HashMap<usize, SchedEntry> = HashMap::new();
        entries.insert(
            0,
            SchedEntry {
                pri: 100,
                ready: false,
            },
        );
        entries.insert(
            1,
            SchedEntry {
                pri: 50,
                ready: false,
            },
        );
        assert_eq!(pick_next(&entries, None), None);
    }

    // ---- reschedule_yield (osYieldThread) sync-path coverage ----

    #[test]
    fn wake_bumps_wake_gen() {
        // wake() must bump wake_gen so a sole-runnable reschedule_yield waiter is released.
        // pri-0 key: registered but never dispatched, so it can't perturb the global scheduler.
        let s = scheduler();
        let key = 0x9E6_9E60usize;
        s.register(key, 0);
        let before = s.inner.lock().unwrap().wake_gen;
        wake(key);
        let after = s.inner.lock().unwrap().wake_gen;
        assert_ne!(before, after, "wake() must bump wake_gen to release a sole-runnable yielder");
    }

    static RY_PEER_Q: AtomicUsize = AtomicUsize::new(0);
    static RY_PEER_RAN: AtomicBool = AtomicBool::new(false);
    static RY_CALLER_DONE: AtomicBool = AtomicBool::new(false);

    extern "C" fn ry_peer(_arg: *mut std::os::raw::c_void) {
        // Runs once, then parks in recv(BLOCK) → not READY until the test's tokenless send wakes it.
        let _ = crate::ultra::mesg::recv(RY_PEER_Q.load(Ordering::SeqCst), 1);
        RY_PEER_RAN.store(true, Ordering::SeqCst);
    }

    extern "C" fn ry_caller(_arg: *mut std::os::raw::c_void) {
        // With the peer parked this thread is the sole runnable choice; each osYieldThread lands
        // in reschedule_yield's condvar wait. It must resume and hand off once the peer is woken.
        while !RY_PEER_RAN.load(Ordering::SeqCst) {
            crate::ultra::thread::HLXThreadYield();
        }
        RY_CALLER_DONE.store(true, Ordering::SeqCst);
    }

    #[test]
    fn sole_runnable_yielder_progresses_when_peer_woken() {
        // End-to-end liveness of the sole-runnable branch (the wait_for_audio_frames shape): a
        // blocked yielder must progress when a tokenless waker marks a higher-priority peer READY.
        // The timeout is raised to 10s so only an actual wake — not timeout polling — can release
        // the caller within the ~1s budget below, so a broken wake path fails the test.
        // RAII restore so an assertion panic can't leave the global override at 10s.
        struct RestoreTimeout(u64);
        impl Drop for RestoreTimeout {
            fn drop(&mut self) {
                TEST_YIELD_TIMEOUT_MS.store(self.0, Ordering::SeqCst);
            }
        }
        let _restore = RestoreTimeout(TEST_YIELD_TIMEOUT_MS.swap(10_000, Ordering::SeqCst));
        let sole0 = TEST_SOLE_WAITS.load(Ordering::SeqCst);

        let mut qbuf = [0u8; 64];
        let q = qbuf.as_mut_ptr() as usize;
        crate::ultra::mesg::create(q, 1);
        RY_PEER_Q.store(q, Ordering::SeqCst);
        RY_PEER_RAN.store(false, Ordering::SeqCst);
        RY_CALLER_DONE.store(false, Ordering::SeqCst);

        // Peer (pri 100) starts first: runs, then parks in recv(BLOCK).
        let mut ps = [0u8; 128];
        let pt = ps.as_mut_ptr() as *mut std::os::raw::c_void;
        let peer_key = pt as usize;
        crate::ultra::thread::HLXThreadCreate(
            pt, 200, ry_peer, std::ptr::null_mut(), std::ptr::null_mut(), 100,
        );
        crate::ultra::thread::HLXThreadStart(pt);

        // Gate on the peer being GENUINELY parked in recv(BLOCK) before proceeding: only then is
        // the later send() guaranteed to hit a waiting receiver and invoke `wake` (else send could
        // enqueue before recv, no wake fires, and the test would pass without exercising wake_gen).
        let mut peer_parked = false;
        for _ in 0..600 {
            if is_blocked(peer_key) {
                peer_parked = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(peer_parked, "peer never parked in recv(BLOCK)");

        // Caller (pri 10) becomes running and, with the peer parked, is the sole runnable choice,
        // so its osYieldThread lands in reschedule_yield's condvar wait.
        let mut cs = [0u8; 128];
        let ct = cs.as_mut_ptr() as *mut std::os::raw::c_void;
        let caller_key = ct as usize;
        crate::ultra::thread::HLXThreadCreate(
            ct, 201, ry_caller, std::ptr::null_mut(), std::ptr::null_mut(), 10,
        );
        crate::ultra::thread::HLXThreadStart(ct);

        // Wait until the caller has entered the sole-wait before sending, so the send actually
        // exercises wake-driven release (not the caller taking the peer-ready arm on a first yield).
        let mut armed = false;
        for _ in 0..600 {
            if TEST_SOLE_WAITS.load(Ordering::SeqCst) > sole0 {
                armed = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(armed, "caller never entered the sole-wait branch");
        // Armed + 10s timeout ⇒ it is provably parked in the condvar wait, not busy-completing.
        assert!(!RY_CALLER_DONE.load(Ordering::SeqCst), "caller not blocked in the sole-wait");

        // Tokenless waker: send() wakes the peer; the caller must be released by that wake, hand
        // off to it, and finish — well within the ~1s budget (vs the 10s timeout).
        crate::ultra::mesg::send(q, 0x1, 0);
        let mut done = false;
        for _ in 0..200 {
            if RY_CALLER_DONE.load(Ordering::SeqCst) {
                done = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            done,
            "sole-runnable yielder was not released by the wake within 1s (only the 10s timeout \
             could have — i.e. wake/notify-driven release is broken)",
        );

        // Teardown: wait until both threads have retired via on_exit before their keys leave scope.
        let both_exited = || {
            let ex = TEST_EXITED.lock().unwrap();
            ex.contains(&peer_key) && ex.contains(&caller_key)
        };
        for _ in 0..600 {
            if both_exited() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(both_exited(), "guest threads did not both reach on_exit");
    }
}
