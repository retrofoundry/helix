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

struct SchedInner {
    entries: HashMap<usize, SchedEntry>,
    running: Option<usize>,
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

    /// A thread's entry function returned: retire it and pass the token on.
    pub fn on_exit(&self, key: usize) {
        let mut g = self.inner.lock().unwrap();
        if let Some(entry) = g.entries.get_mut(&key) {
            entry.ready = false;
        }
        g.running = pick_next(&g.entries, None);
        self.cv.notify_all();
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
}
