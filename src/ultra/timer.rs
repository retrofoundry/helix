//! Host-monotonic clock scaled to the N64 CP0 COUNT rate (osGetCount/osGetTime read from here).
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

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
    let elapsed = cycles_from_nanos(nanos, OS_CPU_COUNTER);
    TIME_OFFSET.store(t.wrapping_sub(elapsed), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

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
        HLXSetTime(1_000_000);
        let a = HLXGetTime();
        assert!(a >= 1_000_000, "got {a}");
        let b = HLXGetTime();
        assert!(b >= a, "time went backwards: {a} -> {b}");
    }
}
