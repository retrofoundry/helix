//! Host-monotonic clock scaled to the N64 CPU clock (osClockRate).
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// N64 CPU clock rate — include/PR/os.h OS_CLOCK_RATE (62.5 MHz).
pub const OS_CLOCK_RATE: u64 = 62_500_000;

/// Fixed instant the runtime started; osGetTime is measured from here.
static EPOCH: OnceLock<Instant> = OnceLock::new();
/// Bias applied by osSetTime so a later osGetTime reads back the set value.
static TIME_OFFSET: AtomicU64 = AtomicU64::new(0);

fn epoch() -> Instant {
    *EPOCH.get_or_init(Instant::now)
}

/// Scale host nanoseconds to osClockRate cycles. Pure + testable.
pub fn cycles_from_nanos(nanos: u128, clock_rate: u64) -> u64 {
    (nanos * clock_rate as u128 / 1_000_000_000u128) as u64
}

fn now_cycles() -> u64 {
    let nanos = epoch().elapsed().as_nanos();
    TIME_OFFSET
        .load(Ordering::Relaxed)
        .wrapping_add(cycles_from_nanos(nanos, OS_CLOCK_RATE))
}

#[no_mangle]
pub extern "C" fn HLXGetTime() -> u64 {
    now_cycles()
}

#[no_mangle]
pub extern "C" fn HLXGetCount() -> u32 {
    // osGetCount is the low 32 bits of the same scaled clock.
    now_cycles() as u32
}

#[no_mangle]
pub extern "C" fn HLXSetTime(t: u64) {
    // Re-bias so the next HLXGetTime returns ~t (osSetTime semantics).
    let nanos = epoch().elapsed().as_nanos();
    let elapsed = cycles_from_nanos(nanos, OS_CLOCK_RATE);
    TIME_OFFSET.store(t.wrapping_sub(elapsed), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_one_second_to_osclockrate() {
        // 1 s of host time == osClockRate cycles (62.5M).
        assert_eq!(cycles_from_nanos(1_000_000_000, OS_CLOCK_RATE), 62_500_000);
    }

    #[test]
    fn scales_half_second() {
        assert_eq!(cycles_from_nanos(500_000_000, OS_CLOCK_RATE), 31_250_000);
    }

    #[test]
    fn zero_elapsed_is_zero() {
        assert_eq!(cycles_from_nanos(0, OS_CLOCK_RATE), 0);
    }

    #[test]
    fn set_time_then_get_time_is_monotonic_from_base() {
        HLXSetTime(1_000_000);
        let a = HLXGetTime();
        assert!(a >= 1_000_000, "got {a}");
        let b = HLXGetTime();
        assert!(b >= a, "time went backwards: {a} -> {b}");
    }
}
