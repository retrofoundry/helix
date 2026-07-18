use std::os::raw::c_void;
use std::sync::{Mutex, OnceLock};

use crate::ultra::mesg;

const NUM_EVENTS: usize = 32;

// Numeric OS_EVENT_* selectors (include/PR/os_message.h). Exported so other modules can
// import them and call `event::post(..)` without redefining the constants.
pub const OS_EVENT_SP: i32 = 4; // SP task done
pub const OS_EVENT_SI: i32 = 5; // SI (controller) read done
pub const OS_EVENT_DP: i32 = 9; // DP full-sync done

// event index -> (queue key, message value)
static EVENTS: OnceLock<Mutex<[Option<(usize, usize)>; NUM_EVENTS]>> = OnceLock::new();

fn table() -> &'static Mutex<[Option<(usize, usize)>; NUM_EVENTS]> {
    EVENTS.get_or_init(|| Mutex::new([None; NUM_EVENTS]))
}

#[no_mangle]
pub extern "C" fn HLXEventSetMesg(event: i32, mq: *mut c_void, msg: *mut c_void) {
    let idx = event as usize;
    if idx >= NUM_EVENTS {
        return;
    }
    table().lock().unwrap()[idx] = Some((mq as usize, msg as usize));
}

/// Post the message registered for `event` to its queue (tail order, NOBLOCK).
/// Rust callers (SP/DP completion, AI) use this directly; the FFI
/// `HLXEventPost` is a thin wrapper.
pub fn post(event: i32) {
    let idx = event as usize;
    let entry = if idx < NUM_EVENTS {
        table().lock().unwrap()[idx]
    } else {
        None
    };
    if let Some((key, msg)) = entry {
        // osSendMesg tail order, NOBLOCK — host injectors never park.
        mesg::send(key, msg, 0);
    }
}

#[no_mangle]
pub extern "C" fn HLXEventPost(event: i32) {
    post(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ultra::mesg;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    // The `EVENTS` table is a process-global `static`. In production exactly one caller
    // registers a given OS_EVENT_* selector, so a post always routes to that caller's queue.
    // In the test binary, however, `post_delivers_registered_message` and
    // `si_post_unblocks_waiter` BOTH register OS_EVENT_SI, and the default runner executes
    // them concurrently — so one test can overwrite `EVENTS[OS_EVENT_SI]` between the other's
    // register and post, misrouting the post to the wrong queue. This lock serializes every
    // test that touches the shared OS_EVENT_SI slot, restoring the one-registrant-per-event
    // invariant the production code assumes. (Poison-tolerant: a panic in one test must not
    // cascade and mask the real failure in another.)
    static EVENT_SI_GUARD: Mutex<()> = Mutex::new(());

    fn lock_si_slot() -> std::sync::MutexGuard<'static, ()> {
        EVENT_SI_GUARD.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn post_delivers_registered_message() {
        let _guard = lock_si_slot();
        let mut backing = [0u8; 64];
        let mq = backing.as_mut_ptr() as *mut std::os::raw::c_void;
        let mut msgbuf: [*mut std::os::raw::c_void; 1] = [ptr::null_mut()];
        mesg::HLXMesgQueueCreate(mq, msgbuf.as_mut_ptr(), 1);
        HLXEventSetMesg(OS_EVENT_SI, mq, 0x5150 as *mut _);
        HLXEventPost(OS_EVENT_SI);
        let (ret, m) = mesg::recv(mq as usize, 0);
        assert_eq!(ret, 0);
        assert_eq!(m, 0x5150);
    }

    #[test]
    fn post_unregistered_event_is_noop() {
        HLXEventPost(6); // OS_EVENT_AI, never registered -> must not panic
    }

    static SI_QKEY: AtomicUsize = AtomicUsize::new(0);
    static SI_GOT: AtomicBool = AtomicBool::new(false);

    extern "C" fn si_waiter(_arg: *mut std::os::raw::c_void) {
        let key = SI_QKEY.load(Ordering::SeqCst);
        let (ret, _m) = mesg::recv(key, 1); // BLOCK on gSIEventMesgQueue
        assert_eq!(ret, 0);
        SI_GOT.store(true, Ordering::SeqCst);
    }

    #[test]
    fn si_post_unblocks_waiter() {
        // Hold the shared OS_EVENT_SI slot for the whole test: guarantees our registration
        // below is the one HLXEventPost(OS_EVENT_SI) reads, so the wake is delivered to OUR
        // queue and not misrouted to a concurrently-running SI test's queue.
        let _guard = lock_si_slot();

        let mut backing = [0u8; 64];
        let mq = backing.as_mut_ptr() as *mut std::os::raw::c_void;
        let mut msgbuf: [*mut std::os::raw::c_void; 1] = [ptr::null_mut()];
        mesg::HLXMesgQueueCreate(mq, msgbuf.as_mut_ptr(), 1);
        SI_QKEY.store(mq as usize, Ordering::SeqCst);
        SI_GOT.store(false, Ordering::SeqCst);
        HLXEventSetMesg(OS_EVENT_SI, mq, ptr::null_mut()); // main.c:117 registers NULL msg

        let mut tstore = [0u8; 128];
        let t = tstore.as_mut_ptr() as *mut std::os::raw::c_void;
        let tid = t as usize;
        crate::ultra::thread::HLXThreadCreate(
            t,
            5,
            si_waiter,
            ptr::null_mut(),
            ptr::null_mut(),
            10,
        );
        crate::ultra::thread::HLXThreadStart(t);

        // Deterministically confirm the guest actually PARKED in recv(BLOCK) on the SI queue
        // before we post — modeled on mesg::tests' F3 poll-until-state technique instead of a
        // fixed sleep. `sched::is_blocked(tid)` becomes true once the guest ran `set_blocked`
        // + `park_self` (registered & !ready). The bound is generous (~10s) so a loaded
        // multi-threaded runner can never false-fail; a normal run trips it in the first poll
        // or two. SI_GOT must stay false throughout: recv can only return AFTER the post below,
        // so observing it true here would mean the guest never genuinely blocked.
        const POLL_MAX: usize = 2000; // 2000 * 5ms = ~10s ceiling
        let mut parked = false;
        for _ in 0..POLL_MAX {
            if crate::ultra::sched::is_blocked(tid) {
                parked = true;
                break;
            }
            assert!(
                !SI_GOT.load(Ordering::SeqCst),
                "waiter returned from recv before it was ever observed parked (no genuine block)"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            parked,
            "guest waiter never parked in recv(BLOCK) on the SI queue"
        );
        assert!(
            !SI_GOT.load(Ordering::SeqCst),
            "waiter unblocked before OS_EVENT_SI was posted"
        );

        HLXEventPost(OS_EVENT_SI); // os_cont.c posts this after sampling

        // Bounded wait for the woken guest to be OS-scheduled out of park_self, pop the
        // message, and record it. The same generous ceiling absorbs pure OS-scheduling latency
        // (and the bounded delay from the globally-shared cooperative scheduler being contended
        // by other tests' guest threads) without masking a genuine wake-delivery failure.
        let mut woke = false;
        for _ in 0..POLL_MAX {
            if SI_GOT.load(Ordering::SeqCst) {
                woke = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(woke, "OS_EVENT_SI did not unblock the waiter");
    }
}
