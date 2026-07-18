//! OSThread(*mut c_void) -> host std::thread mapping. Entry is a native extern"C"
//! fn(*mut c_void); the guest stack pointer is IGNORED (nothing aliases the game's
//! stack arrays as data). Only the run-token holder executes guest code.

use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::{Mutex, OnceLock};

use crate::ultra::sched::{self, scheduler};

/// Raw guest pointer carried into a spawned host thread. Safe because the
/// cooperative-priority scheduler guarantees at most one guest thread touches
/// guest memory at a time.
struct SendPtr(*mut c_void);
unsafe impl Send for SendPtr {}

type Entry = extern "C" fn(*mut c_void);

struct ThreadRecord {
    entry: Entry,
    arg: SendPtr,
    started: bool,
}

static REGISTRY: OnceLock<Mutex<HashMap<usize, ThreadRecord>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<usize, ThreadRecord>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The OSThread* key this host thread runs guest code for, as an Option (None on the
/// process main / injector threads). CURRENT_ID now lives in `sched`; sched
/// reports non-guest threads as 0, so 0 -> None here (a real OSThread* is never null).
fn current_id() -> Option<usize> {
    match sched::current_id() {
        0 => None,
        key => Some(key),
    }
}

#[no_mangle]
pub extern "C" fn HLXThreadCreate(
    t: *mut c_void,
    _id: i32,
    entry: Entry,
    arg: *mut c_void,
    _sp: *mut c_void,
    pri: i32,
) {
    let key = t as usize;
    registry().lock().unwrap().insert(
        key,
        ThreadRecord {
            entry,
            arg: SendPtr(arg),
            started: false,
        },
    );
    scheduler().register(key, pri);
}

#[no_mangle]
pub extern "C" fn HLXThreadStart(t: *mut c_void) {
    let key = t as usize;
    let (entry, arg) = {
        let mut reg = registry().lock().unwrap();
        let rec = reg
            .get_mut(&key)
            .expect("HLXThreadStart: OSThread was never created");
        if rec.started {
            return;
        }
        rec.started = true;
        (rec.entry, SendPtr(rec.arg.0))
    };

    // One host thread per OSThread. It records its identity in sched, parks until it holds
    // the run token, then runs guest code. thread3_main / thread1_idle never return, so
    // on_exit is currently unreached.
    std::thread::spawn(move || {
        // Force capture of the whole `SendPtr`, not just its `.0` field: Rust 2021's
        // disjoint-field closure capture would otherwise capture `*mut c_void` directly,
        // bypassing SendPtr's `unsafe impl Send` and failing the `spawn` bound.
        let arg = arg;
        sched::set_current_id(key);
        scheduler().acquire(key);
        (entry)(arg.0);
        scheduler().on_exit(key);
    });

    // osStartThread is a scheduling point: mark the new thread READY and
    // reschedule from the caller's context. Equal priority does not preempt the caller.
    scheduler().mark_ready(key);
    scheduler().reschedule(current_id());
}

#[no_mangle]
pub extern "C" fn HLXThreadSetPri(t: *mut c_void, pri: i32) {
    // NULL target == the calling (self) thread.
    let key = if t.is_null() {
        current_id().expect("HLXThreadSetPri(NULL): called from a non-guest thread")
    } else {
        t as usize
    };
    scheduler().set_pri(key, pri);
    // Scheduling point: a self-lower to pri 0 parks the caller here (idle deadlock fix).
    scheduler().reschedule(current_id());
}

#[no_mangle]
pub extern "C" fn HLXThreadStop(t: *mut c_void) {
    let key = t as usize;
    scheduler().set_pri(key, 0);
    scheduler().mark_ready(key); // keep it known; pri-0 filter keeps it off-CPU
}
