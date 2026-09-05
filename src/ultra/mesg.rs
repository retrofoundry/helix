use std::collections::{HashMap, VecDeque};
use std::os::raw::c_void;
use std::sync::{Arc, Mutex, OnceLock};

use crate::ultra::sched;

const OS_MESG_BLOCK: i32 = 1; // include/PR/os_message.h: OS_MESG_NOBLOCK=0, OS_MESG_BLOCK=1

struct Queue {
    buf: Vec<usize>, // stores OSMesg (void*) as usize
    first: usize,
    valid: usize,
    cap: usize,
    recv_waiters: VecDeque<usize>, // OSThread keys blocked in recv (mtqueue)
    send_waiters: VecDeque<usize>, // OSThread keys blocked in send (fullqueue)
}

impl Queue {
    fn push_tail(&mut self, m: usize) {
        let i = (self.first + self.valid) % self.cap;
        self.buf[i] = m;
        self.valid += 1;
    }
    fn pop_head(&mut self) -> usize {
        let m = self.buf[self.first];
        self.first = (self.first + 1) % self.cap;
        self.valid -= 1;
        m
    }
    /// Is message `m` already sitting unconsumed in the ring? Used by the VI
    /// coalescing send to bound outstanding retraces to one.
    fn contains(&self, m: usize) -> bool {
        (0..self.valid).any(|k| self.buf[(self.first + k) % self.cap] == m)
    }
}

static QUEUES: OnceLock<Mutex<HashMap<usize, Arc<Mutex<Queue>>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<usize, Arc<Mutex<Queue>>>> {
    QUEUES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lookup(key: usize) -> Arc<Mutex<Queue>> {
    registry()
        .lock()
        .unwrap()
        .get(&key)
        .expect("HLXMesg*: queue used before HLXMesgQueueCreate")
        .clone()
}

pub(crate) fn create(key: usize, count: i32) {
    let cap = (count.max(1)) as usize;
    let q = Queue {
        buf: vec![0usize; cap],
        first: 0,
        valid: 0,
        cap,
        recv_waiters: VecDeque::new(),
        send_waiters: VecDeque::new(),
    };
    registry()
        .lock()
        .unwrap()
        .insert(key, Arc::new(Mutex::new(q)));
}

pub(crate) fn send(key: usize, msg: usize, flag: i32) -> i32 {
    let q = lookup(key);
    loop {
        let mut g = q.lock().unwrap();
        if g.valid < g.cap {
            g.push_tail(msg);
            if let Some(w) = g.recv_waiters.pop_front() {
                sched::wake(w); // atomic mark-READY+wake, under the queue lock
            }
            return 0;
        }
        if flag != OS_MESG_BLOCK {
            // Full ring + NOBLOCK: ordinary osSendMesg drop. Retraces are coalesced at the VI
            // source (send_coalescing), so they can't fill the ring and drop SP/DP completions.
            return -1; // full + NOBLOCK
        }
        // full + BLOCK: enqueue self and park until a recv frees a slot
        let me = sched::current_id();
        g.send_waiters.push_back(me);
        sched::set_blocked(me);
        drop(g);
        sched::park_self();
        // loop: retry the tail-insert
    }
}

/// Coalescing NOBLOCK send for the VI retrace: if `msg` is already pending in the ring, suppress
/// this post so at most one retrace is ever queued; otherwise send normally. Keeps the free-running
/// VI clock from filling `gIntrMesgQueue` and dropping the non-droppable SP/DP completions. The
/// check+insert is under the queue lock, so it can't race a concurrent post/recv.
pub(crate) fn send_coalescing(key: usize, msg: usize) -> i32 {
    let q = lookup(key);
    let mut g = q.lock().unwrap();
    if g.contains(msg) {
        return 0; // a retrace is already pending: coalesce (drop at source)
    }
    if g.valid < g.cap {
        g.push_tail(msg);
        if let Some(w) = g.recv_waiters.pop_front() {
            sched::wake(w);
        }
        return 0;
    }
    -1 // full of non-retrace messages (rare): ordinary NOBLOCK drop
}

pub(crate) fn recv(key: usize, flag: i32) -> (i32, usize) {
    let q = lookup(key);
    loop {
        let mut g = q.lock().unwrap();
        if g.valid > 0 {
            let m = g.pop_head();
            if let Some(s) = g.send_waiters.pop_front() {
                sched::wake(s);
            }
            return (0, m);
        }
        if flag != OS_MESG_BLOCK {
            return (-1, 0); // empty + NOBLOCK
        }
        // empty + BLOCK: enqueue self and park (mark-BLOCKED under the queue lock)
        let me = sched::current_id();
        g.recv_waiters.push_back(me);
        sched::set_blocked(me);
        drop(g);
        sched::park_self(); // releases token, returns when woken by a sender
                            // loop: re-acquire the queue lock and retry the pop
    }
}

#[no_mangle]
pub extern "C" fn HLXMesgQueueCreate(mq: *mut c_void, _msgbuf: *mut *mut c_void, count: i32) {
    create(mq as usize, count);
}

#[no_mangle]
pub extern "C" fn HLXMesgSend(mq: *mut c_void, msg: *mut c_void, flag: i32) -> i32 {
    send(mq as usize, msg as usize, flag)
}

#[no_mangle]
pub extern "C" fn HLXMesgRecv(mq: *mut c_void, msg_out: *mut *mut c_void, flag: i32) -> i32 {
    let (ret, m) = recv(mq as usize, flag);
    if ret == 0 && !msg_out.is_null() {
        unsafe { *msg_out = m as *mut c_void };
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn qkey(buf: &mut [u8]) -> usize {
        buf.as_mut_ptr() as usize
    }

    static T2_QKEY: AtomicUsize = AtomicUsize::new(0);
    static T2_GOT: AtomicBool = AtomicBool::new(false);

    extern "C" fn t2_waiter(_arg: *mut std::os::raw::c_void) {
        let key = T2_QKEY.load(Ordering::SeqCst);
        let (ret, m) = recv(key, 1); // BLOCK
        assert_eq!(ret, 0);
        assert_eq!(m, 0x77);
        T2_GOT.store(true, Ordering::SeqCst);
    }

    #[test]
    fn block_recv_no_lost_wakeup() {
        let mut backing = [0u8; 64];
        let key = backing.as_mut_ptr() as usize;
        create(key, 1);
        T2_QKEY.store(key, Ordering::SeqCst);
        T2_GOT.store(false, Ordering::SeqCst);

        let mut tstore = [0u8; 128];
        let t = tstore.as_mut_ptr() as *mut std::os::raw::c_void;
        crate::ultra::thread::HLXThreadCreate(
            t,
            5,
            t2_waiter,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            10,
        );
        crate::ultra::thread::HLXThreadStart(t);

        // Race: send immediately, before/around the waiter reaching park_self.
        send(key, 0x77, 0);

        for _ in 0..200 {
            if T2_GOT.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            T2_GOT.load(Ordering::SeqCst),
            "blocked recv never woke (lost wakeup)"
        );
    }

    static BS_QKEY: AtomicUsize = AtomicUsize::new(0);
    static BS_SENT: AtomicBool = AtomicBool::new(false);

    extern "C" fn bs_sender(_arg: *mut std::os::raw::c_void) {
        // Queue is full: this BLOCK send must park until the driver frees a slot, then
        // tail-insert and return success.
        let ret = send(BS_QKEY.load(Ordering::SeqCst), 0x22, 1); // BLOCK
        assert_eq!(ret, 0);
        BS_SENT.store(true, Ordering::SeqCst);
    }

    #[test]
    fn block_send_parks_until_slot_frees_then_fifo() {
        // Exercises the full+BLOCK send park path (mesg::send): unreached by SM64 (it issues no
        // blocking sends), but a core primitive for guests that use osSendMesg(OS_MESG_BLOCK).
        // cap-2 so the tail-insert is observable — a retained older message must precede the woken
        // sender's.
        use crate::ultra::sched::{is_blocked, TEST_EXITED};

        let mut backing = [0u8; 64];
        let key = backing.as_mut_ptr() as usize;
        create(key, 2);
        assert_eq!(send(key, 0x11, 0), 0, "fills slot 1");
        assert_eq!(send(key, 0x44, 0), 0, "fills slot 2 — queue now full");
        assert_eq!(send(key, 0x33, 0), -1, "NOBLOCK send now fails: queue full");

        BS_QKEY.store(key, Ordering::SeqCst);
        BS_SENT.store(false, Ordering::SeqCst);

        let mut tstore = [0u8; 128];
        let t = tstore.as_mut_ptr() as *mut std::os::raw::c_void;
        let sender_key = t as usize;
        crate::ultra::thread::HLXThreadCreate(
            t,
            5,
            bs_sender,
            ptr::null_mut(),
            ptr::null_mut(),
            10,
        );
        crate::ultra::thread::HLXThreadStart(t);

        // Deterministically wait until the sender has actually PARKED in the full+BLOCK branch
        // (set_blocked) before freeing a slot — otherwise it could take the non-full fast path
        // after the drain and the test would pass without ever exercising the park.
        let mut parked = false;
        for _ in 0..400 {
            if is_blocked(sender_key) {
                parked = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(parked, "BLOCK send never parked on the full queue");
        assert!(
            !BS_SENT.load(Ordering::SeqCst),
            "a parked send must not have completed"
        );

        // Free one slot. FIFO: 0x11 (head) drains first; 0x44 stays queued.
        let (ret, m) = recv(key, 0);
        assert_eq!(ret, 0);
        assert_eq!(m, 0x11, "FIFO head drains first");

        // The parked sender wakes and TAIL-inserts 0x22 (after the retained 0x44).
        for _ in 0..200 {
            if BS_SENT.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            BS_SENT.load(Ordering::SeqCst),
            "parked sender never woke after a slot freed"
        );

        let (r1, m1) = recv(key, 0);
        assert_eq!(r1, 0);
        assert_eq!(
            m1, 0x44,
            "retained older message drains before the woken sender's"
        );
        let (r2, m2) = recv(key, 0);
        assert_eq!(r2, 0);
        assert_eq!(
            m2, 0x22,
            "woken sender's message was tail-inserted, after 0x44"
        );

        // Teardown: wait for the sender to retire so its stack-derived key leaves the registries.
        for _ in 0..200 {
            if TEST_EXITED.lock().unwrap().contains(&sender_key) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    static F3_Q: AtomicUsize = AtomicUsize::new(0);
    static F3_Q2: AtomicUsize = AtomicUsize::new(0);
    static F3_SEQ: AtomicUsize = AtomicUsize::new(0);
    static F3_WAKER_YIELD_SEQ: AtomicUsize = AtomicUsize::new(usize::MAX);
    static F3_TARGET_RESUME_SEQ: AtomicUsize = AtomicUsize::new(usize::MAX);
    static F3_TARGET_DONE: AtomicBool = AtomicBool::new(false);

    extern "C" fn f3_target(_arg: *mut std::os::raw::c_void) {
        // pri-100: park in recv(BLOCK) until the pri-10 waker sends, then stamp resume order.
        let (ret, m) = recv(F3_Q.load(Ordering::SeqCst), 1); // BLOCK
        assert_eq!(ret, 0);
        assert_eq!(m, 0x99);
        F3_TARGET_RESUME_SEQ.store(F3_SEQ.fetch_add(1, Ordering::SeqCst), Ordering::SeqCst);
        F3_TARGET_DONE.store(true, Ordering::SeqCst);
    }

    extern "C" fn f3_waker(_arg: *mut std::os::raw::c_void) {
        // pri-10 holds the token: wake the higher-priority target, then keep running to
        // our OWN next park_self (recv on Q2). The target must not resume until we park.
        send(F3_Q.load(Ordering::SeqCst), 0x99, 0); // wakes target; deferred (we hold the token)
        F3_WAKER_YIELD_SEQ.store(F3_SEQ.fetch_add(1, Ordering::SeqCst), Ordering::SeqCst);
        let _ = recv(F3_Q2.load(Ordering::SeqCst), 1); // BLOCK -> park_self; only now may target run
    }

    #[test]
    fn wake_defers_to_token_holder_no_preempt() {
        let mut qbacking = [0u8; 64];
        let mut q2backing = [0u8; 64];
        let q = qbacking.as_mut_ptr() as usize;
        let q2 = q2backing.as_mut_ptr() as usize;
        create(q, 1);
        create(q2, 1);
        F3_Q.store(q, Ordering::SeqCst);
        F3_Q2.store(q2, Ordering::SeqCst);
        F3_SEQ.store(0, Ordering::SeqCst);
        F3_WAKER_YIELD_SEQ.store(usize::MAX, Ordering::SeqCst);
        F3_TARGET_RESUME_SEQ.store(usize::MAX, Ordering::SeqCst);
        F3_TARGET_DONE.store(false, Ordering::SeqCst);

        // Start the high-priority target first so it parks in recv(BLOCK)...
        let mut tstore = [0u8; 128];
        let tt = tstore.as_mut_ptr() as *mut std::os::raw::c_void;
        crate::ultra::thread::HLXThreadCreate(
            tt,
            6,
            f3_target,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            100,
        );
        crate::ultra::thread::HLXThreadStart(tt);

        // ...then the low-priority waker (the token holder that performs the send).
        let mut wstore = [0u8; 128];
        let ww = wstore.as_mut_ptr() as *mut std::os::raw::c_void;
        crate::ultra::thread::HLXThreadCreate(
            ww,
            7,
            f3_waker,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            10,
        );
        crate::ultra::thread::HLXThreadStart(ww);

        for _ in 0..400 {
            if F3_TARGET_DONE.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            F3_TARGET_DONE.load(Ordering::SeqCst),
            "pri-100 target never resumed"
        );

        let wy = F3_WAKER_YIELD_SEQ.load(Ordering::SeqCst);
        let tr = F3_TARGET_RESUME_SEQ.load(Ordering::SeqCst);
        assert_ne!(
            wy,
            usize::MAX,
            "pri-10 waker never reached its next park_self"
        );
        assert_ne!(tr, usize::MAX, "pri-100 target never recorded its resume");
        // The pri-10 token holder reaches its next park_self BEFORE the pri-100 target
        // resumes: proof that wake() defers instead of preempting mid-execution (F3).
        assert!(
            wy < tr,
            "pri-100 target preempted the pri-10 token holder (F3 data race)"
        );

        // Cleanup: release the parked waker so its guest thread exits before the next test.
        send(q2, 0xEE, 0);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    #[test]
    fn noblock_send_recv_fifo() {
        let mut backing = [0u8; 64];
        let key = qkey(&mut backing);
        create(key, 2);
        assert_eq!(send(key, 0xAA, 0), 0);
        assert_eq!(send(key, 0xBB, 0), 0);
        assert_eq!(send(key, 0xCC, 0), -1); // full + NOBLOCK
        assert_eq!(recv(key, 0), (0, 0xAA));
        assert_eq!(recv(key, 0), (0, 0xBB));
        assert_eq!(recv(key, 0), (-1, 0)); // empty + NOBLOCK
    }

    #[test]
    fn ring_wraps_around() {
        let mut backing = [0u8; 64];
        let key = qkey(&mut backing);
        create(key, 2);
        assert_eq!(send(key, 1, 0), 0); // slot 0
        assert_eq!(send(key, 2, 0), 0); // slot 1 (full)
        assert_eq!(recv(key, 0), (0, 1)); // first -> 1
        assert_eq!(send(key, 3, 0), 0); // wraps into slot 0
        assert_eq!(recv(key, 0), (0, 2));
        assert_eq!(recv(key, 0), (0, 3));
    }

    // gIntrMesgQueue message values (src/game/main.c). The retrace is coalesced by the VI
    // clock; SP/DP completions are posted NOBLOCK from the consumer and must never drop.
    const VI: usize = 102; // MESG_VI_VBLANK
    const SP: usize = 100; // MESG_SP_COMPLETE
    const DP: usize = 101; // MESG_DP_COMPLETE

    #[test]
    fn vi_coalesces_to_one_outstanding() {
        // Two coalescing retrace posts leave exactly ONE retrace queued (the second is
        // suppressed, reported as success, not a drop).
        let mut backing = [0u8; 64];
        let key = qkey(&mut backing);
        create(key, 8);
        assert_eq!(send_coalescing(key, VI), 0);
        assert_eq!(
            send_coalescing(key, VI),
            0,
            "suppressed post still reports success"
        );
        assert_eq!(recv(key, 0), (0, VI));
        assert_eq!(recv(key, 0), (-1, 0), "only one retrace was ever queued");
    }

    #[test]
    fn completions_never_dropped_under_vi_saturation() {
        // The real failure: VI spam must not crowd out SP+DP. Coalescing bounds retraces to 1,
        // so both completions always enqueue behind the single retrace.
        let mut backing = [0u8; 256];
        let key = qkey(&mut backing);
        create(key, 16);
        for _ in 0..100 {
            assert_eq!(send_coalescing(key, VI), 0);
        }
        assert_eq!(send(key, SP, 0), 0, "SP_COMPLETE must not drop");
        assert_eq!(send(key, DP, 0), 0, "DP_COMPLETE must not drop");
        // Exactly one retrace, then both completions, then empty.
        assert_eq!(recv(key, 0), (0, VI));
        assert_eq!(recv(key, 0), (0, SP));
        assert_eq!(recv(key, 0), (0, DP));
        assert_eq!(recv(key, 0), (-1, 0));
    }

    #[test]
    fn vi_reenqueues_after_consume() {
        // Liveness: once the consumer drains the pending retrace, the next tick enqueues a
        // fresh one (so thread3 keeps getting woken).
        let mut backing = [0u8; 64];
        let key = qkey(&mut backing);
        create(key, 4);
        assert_eq!(send_coalescing(key, VI), 0);
        assert_eq!(send_coalescing(key, VI), 0); // suppressed while one is pending
        assert_eq!(recv(key, 0), (0, VI)); // consume it
        assert_eq!(send_coalescing(key, VI), 0); // now a fresh retrace enqueues
        assert_eq!(recv(key, 0), (0, VI));
        assert_eq!(recv(key, 0), (-1, 0));
    }

    #[test]
    fn concurrent_vi_between_completions_is_coalesced() {
        // A retrace attempt landing between an SP and its DP is coalesced away (a retrace is
        // already pending), so it never displaces the DP and never reorders the completions.
        let mut backing = [0u8; 256];
        let key = qkey(&mut backing);
        create(key, 16);
        assert_eq!(send_coalescing(key, VI), 0); // retrace pending
        assert_eq!(send(key, SP, 0), 0);
        assert_eq!(send_coalescing(key, VI), 0); // concurrent tick: suppressed
        assert_eq!(send(key, DP, 0), 0);
        assert_eq!(recv(key, 0), (0, VI));
        assert_eq!(recv(key, 0), (0, SP));
        assert_eq!(recv(key, 0), (0, DP));
        assert_eq!(recv(key, 0), (-1, 0), "no second retrace slipped in");
    }

    #[test]
    fn ffi_roundtrip() {
        let mut backing = [0u8; 64];
        let mq = backing.as_mut_ptr() as *mut std::os::raw::c_void;
        let mut msgbuf: [*mut std::os::raw::c_void; 1] = [ptr::null_mut()];
        HLXMesgQueueCreate(mq, msgbuf.as_mut_ptr(), 1);
        assert_eq!(HLXMesgSend(mq, 0x1234 as *mut _, 0), 0);
        let mut out: *mut std::os::raw::c_void = ptr::null_mut();
        assert_eq!(HLXMesgRecv(mq, &mut out as *mut _, 0), 0);
        assert_eq!(out as usize, 0x1234);
    }

    use std::ffi::c_void;

    // Distinct, stable, non-null handle per queue (the Rust core keys state by this pointer).
    fn fake_queue() -> *mut c_void {
        Box::into_raw(Box::new(0u64)) as *mut c_void
    }

    #[test]
    #[allow(unused_unsafe)]
    fn ring_is_fifo_and_caps_at_count_without_overflow() {
        let mq = fake_queue();
        let mut buf = [std::ptr::null_mut::<c_void>(); 16];
        unsafe { HLXMesgQueueCreate(mq, buf.as_mut_ptr(), 16) };
        for i in 1..=16usize {
            assert_eq!(
                unsafe { HLXMesgSend(mq, i as *mut c_void, 0) },
                0,
                "slot {i}"
            );
        }
        assert_eq!(unsafe { HLXMesgSend(mq, 99usize as *mut c_void, 0) }, -1);
        for i in 1..=16usize {
            let mut out: *mut c_void = std::ptr::null_mut();
            assert_eq!(unsafe { HLXMesgRecv(mq, &mut out, 0) }, 0);
            assert_eq!(out as usize, i);
        }
        let mut out: *mut c_void = std::ptr::null_mut();
        assert_eq!(unsafe { HLXMesgRecv(mq, &mut out, 0) }, -1);
    }

    #[test]
    #[allow(unused_unsafe)]
    fn interrupt_burst_sp_drains_before_dp() {
        const MESG_VI: usize = 1;
        const MESG_SP: usize = 2;
        const MESG_DP: usize = 3;
        let mq = fake_queue();
        let mut buf = [std::ptr::null_mut::<c_void>(); 16];
        unsafe { HLXMesgQueueCreate(mq, buf.as_mut_ptr(), 16) };
        for _ in 0..3 {
            assert_eq!(unsafe { HLXMesgSend(mq, MESG_VI as *mut c_void, 0) }, 0);
        }
        assert_eq!(unsafe { HLXMesgSend(mq, MESG_SP as *mut c_void, 0) }, 0);
        assert_eq!(unsafe { HLXMesgSend(mq, MESG_DP as *mut c_void, 0) }, 0);
        for expected in [MESG_VI, MESG_VI, MESG_VI, MESG_SP, MESG_DP] {
            let mut out: *mut c_void = std::ptr::null_mut();
            assert_eq!(unsafe { HLXMesgRecv(mq, &mut out, 0) }, 0);
            assert_eq!(out as usize, expected);
        }
    }
}
