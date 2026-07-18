//! Send+Sync controller-input snapshot: the ONLY gamepad data that crosses threads in the
//! libultra runtime.
//!
//! The [`GamepadManager`](crate::gamepad::manager::GamepadManager) is gilrs-backed and therefore
//! `!Send`; it is owned and pumped EXCLUSIVELY on the winit main thread. Each frame the main thread
//! samples it (`GamepadManager::sample_snapshot`) and `publish`es a plain-data snapshot here. The
//! game's thread5 reads it through the `HLXControllerInit`/`HLXControllerRead` FFI. Because the
//! snapshot is plain `Copy` data behind a `Mutex`, this is sound: the `!Send` manager is never
//! touched off the main thread — the snapshot is the only shared state.

use std::sync::Mutex;

use crate::gamepad::types::OSControllerPad;

/// Plain, `Send + Sync` controller state (player 1). Mirrors the `OSContPad` fields plus the
/// controller-bits byte that `osContInit` reports through `&gControllerBits`.
#[derive(Clone, Copy, Default)]
pub struct GamepadSnapshot {
    pub button: u16,
    pub stick_x: i8,
    pub stick_y: i8,
    pub errno: u8,
    /// `gControllerBits` value: bit N set => controller N present. Single controller => 0 or 1.
    pub bits: u8,
}

static SNAPSHOT: Mutex<GamepadSnapshot> = Mutex::new(GamepadSnapshot {
    button: 0,
    stick_x: 0,
    stick_y: 0,
    errno: 0,
    bits: 0,
});

/// Main thread: overwrite the shared snapshot with a freshly sampled one.
pub fn publish(snapshot: GamepadSnapshot) {
    *SNAPSHOT.lock().unwrap() = snapshot;
}

/// Any thread: read the latest published snapshot (cheap `Copy`).
pub fn load() -> GamepadSnapshot {
    *SNAPSHOT.lock().unwrap()
}

// MARK: - C API (thread5, runtime path only — guarded by HLXRuntimeActive in os_cont.c)

/// Runtime replacement for `GamepadManagerInit` on thread5: report the controller-bits from the
/// latest snapshot WITHOUT touching the `!Send` manager. `bits` is `&gControllerBits`.
#[no_mangle]
pub extern "C" fn HLXControllerInit(bits: *mut u8) -> i32 {
    if !bits.is_null() {
        unsafe { *bits = load().bits };
    }
    0
}

/// Runtime replacement for `GamepadManagerGetReadData` on thread5: copy the latest snapshot pad.
#[no_mangle]
pub extern "C" fn HLXControllerRead(pad: *mut OSControllerPad) {
    if pad.is_null() {
        return;
    }
    let snap = load();
    unsafe {
        (*pad).button = snap.button;
        (*pad).stick_x = snap.stick_x;
        (*pad).stick_y = snap.stick_y;
        (*pad).errno = snap.errno;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The snapshot is a single process-global; serialize the tests that publish into it so a
    // concurrent test can't observe another's value (same rationale as ultra::event's SI slot).
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn publish_then_load_roundtrips() {
        let _g = TEST_LOCK.lock().unwrap();
        publish(GamepadSnapshot {
            button: 0x8000,
            stick_x: 42,
            stick_y: -7,
            errno: 0,
            bits: 1,
        });
        let snap = load();
        assert_eq!(snap.button, 0x8000);
        assert_eq!(snap.stick_x, 42);
        assert_eq!(snap.stick_y, -7);
        assert_eq!(snap.bits, 1);
    }

    #[test]
    fn controller_init_reads_bits_and_tolerates_null() {
        let _g = TEST_LOCK.lock().unwrap();
        publish(GamepadSnapshot {
            bits: 1,
            ..Default::default()
        });
        let mut bits: u8 = 0xFF;
        assert_eq!(HLXControllerInit(&mut bits), 0);
        assert_eq!(bits, 1);
        // Null pointer must not crash.
        assert_eq!(HLXControllerInit(std::ptr::null_mut()), 0);
    }

    #[test]
    fn controller_read_copies_pad_and_tolerates_null() {
        let _g = TEST_LOCK.lock().unwrap();
        publish(GamepadSnapshot {
            button: 0x1000,
            stick_x: 10,
            stick_y: -20,
            errno: 3,
            bits: 1,
        });
        let mut pad = OSControllerPad {
            button: 0,
            stick_x: 0,
            stick_y: 0,
            errno: 0,
        };
        HLXControllerRead(&mut pad);
        assert_eq!(pad.button, 0x1000);
        assert_eq!(pad.stick_x, 10);
        assert_eq!(pad.stick_y, -20);
        assert_eq!(pad.errno, 3);
        // Null pointer must not crash.
        HLXControllerRead(std::ptr::null_mut());
    }
}
