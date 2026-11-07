//! Host-monotonic clock and countdown timers use the N64 CP0 COUNT rate.
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

/// N64 bus clock rate — include/PR/os.h OS_CLOCK_RATE (62.5 MHz). Not the CPU clock (93.75 MHz).
pub const OS_CLOCK_RATE: u64 = 62_500_000;

/// The CP0 COUNT register — what osGetCount/osGetTime actually read — ticks at half the 93.75 MHz
/// CPU clock = 46.875 MHz, i.e. OS_CLOCK_RATE*3/4 (include/PR/os.h OS_CPU_COUNTER). Scaling by
/// OS_CLOCK_RATE instead would run the guest clock ~33% fast (skewing profiling, RNG, timing).
pub const OS_CPU_COUNTER: u64 = OS_CLOCK_RATE * 3 / 4;

/// Fixed instant the runtime started; osGetTime is measured from here.
static EPOCH: OnceLock<Instant> = OnceLock::new();
/// Bias applied by osSetTime so a later osGetTime reads back the set value.
static TIME_OFFSET: AtomicU64 = AtomicU64::new(0);

fn epoch() -> Instant {
    *EPOCH.get_or_init(Instant::now)
}

/// Scale host nanoseconds to COUNT cycles at the given rate. Pure + testable.
pub fn cycles_from_nanos(nanos: u128, clock_rate: u64) -> u64 {
    (nanos * clock_rate as u128 / 1_000_000_000u128) as u64
}

fn now_cycles() -> u64 {
    let nanos = epoch().elapsed().as_nanos();
    TIME_OFFSET
        .load(Ordering::Relaxed)
        .wrapping_add(cycles_from_nanos(nanos, OS_CPU_COUNTER))
}

#[no_mangle]
pub extern "C" fn R64GetTime() -> u64 {
    now_cycles()
}

#[no_mangle]
pub extern "C" fn R64GetCount() -> u32 {
    // osGetCount is the low 32 bits of the same scaled clock.
    now_cycles() as u32
}

#[no_mangle]
pub extern "C" fn R64SetTime(t: u64) {
    // Re-bias so the next R64GetTime returns ~t (osSetTime semantics).
    let nanos = epoch().elapsed().as_nanos();
    let elapsed = cycles_from_nanos(nanos, OS_CPU_COUNTER);
    TIME_OFFSET.store(t.wrapping_sub(elapsed), Ordering::Relaxed);
}

struct Countdown {
    timer: usize,
    deadline: Instant,
    interval: Option<Duration>,
    mq: usize,
    msg: usize,
}

static ARMED: Mutex<Vec<Countdown>> = Mutex::new(Vec::new());
static ARMED_CHANGED: Condvar = Condvar::new();
static TIMER_THREAD: Once = Once::new();

fn duration_from_cycles(cycles: u64) -> Duration {
    let nanos = (cycles % OS_CPU_COUNTER) * 1_000_000_000 / OS_CPU_COUNTER;
    Duration::new(cycles / OS_CPU_COUNTER, nanos as u32)
}

fn service(armed: &mut Vec<Countdown>, now: Instant) {
    while let Some(i) = (0..armed.len())
        .filter(|&i| armed[i].deadline <= now)
        .min_by_key(|&i| armed[i].deadline)
    {
        // Post under ARMED so no message follows a returned osStopTimer (OoT's RCP watchdog relies on
        // it). A full queue drops the post.
        let _ = super::mesg::post_with(armed[i].mq, armed[i].msg, || {});
        // Like libultra, a periodic timer reloads its interval when serviced rather than catching up.
        match armed[i]
            .interval
            .and_then(|interval| now.checked_add(interval))
        {
            Some(deadline) => armed[i].deadline = deadline,
            None => {
                armed.swap_remove(i);
            }
        }
    }
}

fn countdown_loop() {
    let mut armed = ARMED.lock().unwrap();
    loop {
        service(&mut armed, Instant::now());
        armed = match armed.iter().map(|c| c.deadline).min() {
            None => ARMED_CHANGED.wait(armed).unwrap(),
            Some(deadline) => {
                let wait = deadline.saturating_duration_since(Instant::now());
                ARMED_CHANGED.wait_timeout(armed, wait).unwrap().0
            }
        };
    }
}

/// osSetTimer: post `msg` to `mq` after `countdown` COUNT cycles (`interval` when `countdown` is 0),
/// then every `interval` cycles unless it is 0. Setting an armed timer re-arms it.
#[no_mangle]
pub extern "C" fn R64SetTimer(
    timer: *mut c_void,
    countdown: u64,
    interval: u64,
    mq: *mut c_void,
    msg: *mut c_void,
) -> i32 {
    TIMER_THREAD.call_once(|| {
        std::thread::Builder::new()
            .name("reality64-timer".into())
            .spawn(countdown_loop)
            .expect("spawn timer thread");
    });
    let value = if countdown != 0 { countdown } else { interval };
    let mut armed = ARMED.lock().unwrap();
    armed.retain(|c| c.timer != timer as usize);
    if let Some(deadline) = Instant::now().checked_add(duration_from_cycles(value)) {
        armed.push(Countdown {
            timer: timer as usize,
            deadline,
            interval: (interval != 0).then(|| duration_from_cycles(interval)),
            mq: mq as usize,
            msg: msg as usize,
        });
        ARMED_CHANGED.notify_one();
    }
    0
}

/// osStopTimer: disarm `timer`, returning -1 if it was not armed. Its message is never posted after
/// this returns.
#[no_mangle]
pub extern "C" fn R64StopTimer(timer: *mut c_void) -> i32 {
    let mut armed = ARMED.lock().unwrap();
    let before = armed.len();
    armed.retain(|c| c.timer != timer as usize);
    if armed.len() < before {
        0
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cycles(ms: u64) -> u64 {
        OS_CPU_COUNTER * ms / 1000
    }

    fn key(n: usize) -> *mut c_void {
        (0x7140_0000 + n * 0x100) as *mut c_void
    }

    fn drain(mq: *mut c_void, count: usize, within: Duration) -> Vec<usize> {
        let end = Instant::now() + within;
        let mut got = Vec::new();
        while got.len() < count {
            match crate::ultra::mesg::recv(mq as usize, 0) {
                (0, msg) => got.push(msg),
                _ if Instant::now() >= end => break,
                _ => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        got
    }

    #[test]
    fn one_shot_posts_once_after_its_countdown() {
        let (timer, mq) = (key(0), key(1));
        crate::ultra::mesg::create(mq as usize, 4);
        let start = Instant::now();
        assert_eq!(R64SetTimer(timer, cycles(100), 0, mq, 7 as *mut c_void), 0);
        assert!(drain(mq, 1, Duration::from_millis(20)).is_empty());
        assert_eq!(drain(mq, 2, Duration::from_millis(1000)), [7]);
        assert!(start.elapsed() >= Duration::from_millis(100));
        assert_eq!(R64StopTimer(timer), -1);
    }

    #[test]
    fn stopped_timer_never_posts() {
        let (timer, mq) = (key(2), key(3));
        crate::ultra::mesg::create(mq as usize, 4);
        R64SetTimer(timer, cycles(200), 0, mq, 9 as *mut c_void);
        assert_eq!(R64StopTimer(timer), 0);
        assert!(drain(mq, 1, Duration::from_millis(400)).is_empty());
    }

    #[test]
    fn interval_repeats_and_zero_countdown_starts_at_interval() {
        let (timer, mq) = (key(4), key(5));
        crate::ultra::mesg::create(mq as usize, 16);
        R64SetTimer(timer, 0, cycles(10), mq, 3 as *mut c_void);
        let got = drain(mq, 3, Duration::from_millis(2000));
        assert_eq!(R64StopTimer(timer), 0);
        assert_eq!(got, [3, 3, 3]);
    }

    #[test]
    fn rearming_replaces_the_previous_countdown() {
        let (timer, mq) = (key(6), key(7));
        crate::ultra::mesg::create(mq as usize, 4);
        R64SetTimer(
            timer,
            cycles(200),
            0,
            mq,
            std::ptr::without_provenance_mut(1),
        );
        R64SetTimer(
            timer,
            cycles(250),
            0,
            mq,
            std::ptr::without_provenance_mut(2),
        );
        assert_eq!(drain(mq, 2, Duration::from_millis(600)), [2]);
    }

    fn countdown(
        mq: *mut c_void,
        deadline: Instant,
        interval: Option<Duration>,
        msg: usize,
    ) -> Countdown {
        Countdown {
            timer: msg,
            deadline,
            interval,
            mq: mq as usize,
            msg,
        }
    }

    #[test]
    fn overdue_countdowns_post_in_deadline_order() {
        let mq = key(8);
        crate::ultra::mesg::create(mq as usize, 4);
        let now = Instant::now();
        let mut armed = vec![
            countdown(mq, now - Duration::from_millis(10), None, 2),
            countdown(mq, now - Duration::from_millis(30), None, 1),
        ];
        service(&mut armed, now);
        assert!(armed.is_empty());
        assert_eq!(drain(mq, 3, Duration::ZERO), [1, 2]);
    }

    #[test]
    fn overdue_periodic_countdown_reloads_instead_of_catching_up() {
        let mq = key(9);
        crate::ultra::mesg::create(mq as usize, 16);
        let now = Instant::now();
        let interval = Duration::from_millis(10);
        let mut armed = vec![countdown(
            mq,
            now - Duration::from_secs(1),
            Some(interval),
            5,
        )];
        service(&mut armed, now);
        assert_eq!(armed[0].deadline, now + interval);
        assert_eq!(drain(mq, 2, Duration::ZERO), [5]);
    }

    #[test]
    fn cycles_convert_exactly_without_clamping() {
        assert_eq!(duration_from_cycles(OS_CPU_COUNTER), Duration::from_secs(1));
        assert_eq!(duration_from_cycles(1), Duration::from_nanos(21));
        assert_eq!(
            duration_from_cycles(u64::MAX).as_secs(),
            u64::MAX / OS_CPU_COUNTER
        );
    }

    #[test]
    fn scales_one_second_to_count_rate() {
        // osGetCount/osGetTime tick at OS_CPU_COUNTER (46.875M), not OS_CLOCK_RATE (62.5M).
        assert_eq!(cycles_from_nanos(1_000_000_000, OS_CPU_COUNTER), 46_875_000);
    }

    #[test]
    fn scales_half_second() {
        assert_eq!(cycles_from_nanos(500_000_000, OS_CPU_COUNTER), 23_437_500);
    }

    #[test]
    fn zero_elapsed_is_zero() {
        assert_eq!(cycles_from_nanos(0, OS_CPU_COUNTER), 0);
    }

    #[test]
    fn set_time_then_get_time_is_monotonic_from_base() {
        R64SetTime(1_000_000);
        let a = R64GetTime();
        assert!(a >= 1_000_000, "got {a}");
        let b = R64GetTime();
        assert!(b >= a, "time went backwards: {a} -> {b}");
    }
}
