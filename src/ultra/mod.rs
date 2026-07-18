//! helix libultra runtime: process-global state, scheduler, and native-pointer os* backing.
//! The game runs its ORIGINAL src/game/main.c threads on real host threads; helix owns the
//! "hardware".

pub mod event;
pub mod mesg;
pub mod pi;
pub mod rcp;
pub mod save;
pub mod sched;
pub mod thread;
pub mod timer;
pub mod vi;

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::gamepad::manager::GamepadManager;
use crate::gui::{EventLoopWrapper, Gui};

// The winit EventLoop and the Gui are NOT Send/Sync: keep them pinned to the process
// main thread across the two FFI calls (HLXRuntimeInit stores, HLXRunEventLoop consumes).
thread_local! {
    static EVENT_LOOP: RefCell<Option<EventLoopWrapper>> = const { RefCell::new(None) };
    static GUI: RefCell<Option<Gui<'static>>> = const { RefCell::new(None) };
}

/// Set true once HLXRuntimeInit has run: the game is driving the machine through the
/// libultra runtime. Consulted by the os_cont SI post, distinguishing pre- vs
/// post-HLXRuntimeInit runtime state.
static RUNTIME_ACTIVE: AtomicBool = AtomicBool::new(false);

/// True when the libultra runtime owns frame pacing / input (post-`HLXRuntimeInit`).
pub fn runtime_active() -> bool {
    RUNTIME_ACTIVE.load(Ordering::SeqCst)
}

/// C-callable form of `runtime_active()`. Consumed by the `os_cont.c` SI post to guard the
/// `HLXEventPost(OS_EVENT_SI)` call so controller sampling falls back to the direct
/// GamepadManager path whenever the runtime isn't active.
#[no_mangle]
pub extern "C" fn HLXRuntimeActive() -> bool {
    runtime_active()
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Set by HLXRuntimeInit; lets gui.rs know the runtime owns the loop.
pub fn mark_runtime_active() {
    RUNTIME_ACTIVE.store(true, Ordering::SeqCst);
}

/// Set by the winit CloseRequested handler or a winit `PumpStatus::Exit`, or by the render
/// thread on a fatal construction/loop failure; observed by HLXRunEventLoop's pump loop.
pub fn request_shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}
pub fn shutdown_requested() -> bool {
    SHUTDOWN.load(Ordering::SeqCst)
}

/// Graceful teardown, in order: detach and drop the audio runtime (so no Arie player or
/// callback outlives the process), stop and join the VI retrace clock, then flush EEPROM to
/// disk. Audio is torn down BEFORE the EEPROM flush. Guest host-threads stay parked — the
/// cooperative scheduler has no async preemption; process exit after HLXRunEventLoop returns
/// reclaims them.
pub fn teardown() {
    #[cfg(test)]
    record_teardown_step("audio");
    crate::audio::teardown();

    #[cfg(test)]
    record_teardown_step("vi");
    crate::ultra::vi::stop_clock();

    #[cfg(test)]
    record_teardown_step("save");
    crate::ultra::save::flush();
}

// Per-thread teardown-step log used only by `teardown_runs_audio_before_save_flush`. Cargo
// runs each test on its own thread, so a thread-local isolates concurrent teardown callers.
#[cfg(test)]
thread_local! {
    static TEARDOWN_LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn record_teardown_step(step: &'static str) {
    TEARDOWN_LOG.with(|log| log.borrow_mut().push(step));
}

/// Initialize the runtime on the process MAIN thread: logging, scheduler statics, the
/// RUNTIME_ACTIVE flag, and the main-thread-pinned winit `EventLoopWrapper` + `Gui`.
/// Called first from host_main.c.
#[no_mangle]
pub extern "C" fn HLXRuntimeInit() {
    crate::init();
    // Force the process-global scheduler to exist before any guest thread starts.
    let _ = sched::scheduler();
    mark_runtime_active();

    let wrapper = EventLoopWrapper::new();
    // Leak a GamepadManager so the Gui can hold a `&'static mut` across the two FFI calls
    // (created here, consumed in HLXRunEventLoop). The manager lives for the whole process.
    let gpm: &'static mut GamepadManager = Box::leak(Box::new(GamepadManager::new()));
    let mut gui = Gui::new("Super Mario 64", Some(gpm)).expect("helix: failed to create Gui");

    // Seed the controller snapshot on THIS (main) thread before any guest thread starts, so the
    // first `osContInit` on thread5 observes a plugged-in controller (bits) instead of the
    // default-empty snapshot. The `!Send` manager is only ever pumped here on the main thread.
    gui.sample_gamepads_into_snapshot();

    EVENT_LOOP.with(|slot| *slot.borrow_mut() = Some(wrapper));
    GUI.with(|slot| *slot.borrow_mut() = Some(gui));
    log::info!("helix: runtime initialized");
}

/// Pump winit + gamepad on the process MAIN thread; returns only once shutdown_requested() is
/// observed. Graphics consume + present run OFF this thread, on the dedicated render thread
/// spawned by `Gui::resumed` (via `crate::render::spawn`) the first time the window comes up;
/// guest threads submit gfx tasks straight to that render thread over the `RenderMsg` channel
/// (`ultra::rcp::submit_and_wait`), bypassing this pump entirely. `Gui`/`EventLoopWrapper` were
/// created on THIS thread in HLXRuntimeInit.
#[no_mangle]
pub extern "C" fn HLXRunEventLoop() {
    use winit::platform::pump_events::PumpStatus;

    let mut gui = GUI
        .with(|slot| slot.borrow_mut().take())
        .expect("helix: HLXRunEventLoop must run on the HLXRuntimeInit thread");
    let mut wrapper = EVENT_LOOP
        .with(|slot| slot.borrow_mut().take())
        .expect("helix: HLXRunEventLoop must run on the HLXRuntimeInit thread");

    loop {
        if shutdown_requested() {
            // Wake the render thread (any shutdown path — CloseRequested, a winit Exit, or a
            // fatal render-thread failure), then join before teardown and before `gui` (its
            // Arc<Window>) drops, so nothing presents to a torn-down window. If the window never
            // came up (Exit during readiness), there is no render handle to join.
            crate::ultra::rcp::send_render_control(crate::ultra::rcp::RenderMsg::Shutdown);
            if let Some(handle) = gui.take_render_handle() {
                handle.join();
            }
            teardown();
            return; // unwind to host_main.c main(), which exits the process normally
        }
        // Bounded pump (~2ms once the window exists) so this thread never spins a core nor
        // stalls guest threads for long.
        if let PumpStatus::Exit(_) = gui.pump(&mut wrapper) {
            request_shutdown();
        }
        // Publish this frame's controller snapshot for thread5 (runtime path). `pump` has just
        // drained winit keyboard input into the manager; sampling here keeps the `!Send` manager
        // main-thread-only while thread5 reads the plain snapshot.
        gui.sample_gamepads_into_snapshot();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_and_runtime_active_flags_roundtrip() {
        request_shutdown();
        assert!(shutdown_requested());
        mark_runtime_active();
        assert!(runtime_active());
        teardown(); // must not panic even when EEPROM was never initialized (OnceLock empty)
    }

    #[test]
    fn teardown_runs_audio_before_save_flush() {
        TEARDOWN_LOG.with(|log| log.borrow_mut().clear());
        teardown(); // safe: audio/vi/save teardowns are all no-ops when uninitialized

        TEARDOWN_LOG.with(|log| {
            let log = log.borrow();
            let audio = log
                .iter()
                .position(|step| *step == "audio")
                .expect("audio teardown ran");
            let save = log
                .iter()
                .position(|step| *step == "save")
                .expect("save flush ran");
            assert!(
                audio < save,
                "audio must be torn down before the EEPROM flush"
            );
        });
    }
}
