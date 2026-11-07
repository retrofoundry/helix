//! reality64 libultra runtime: process-global state, scheduler, and native-pointer os* backing.
//! The game runs its ORIGINAL src/game/main.c threads on real host threads; reality64 owns the
//! "hardware".

mod acmd;
pub mod event;
pub mod mesg;
mod njpgdsp;
pub mod pi;
pub mod rcp;
pub mod rom;
pub(crate) mod rsp;
pub mod save;
pub mod sched;
pub mod thread;
pub mod timer;
pub mod vi;

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};

use crate::gamepad::manager::GamepadManager;
use crate::gui::{EventLoopWrapper, Gui};

// The winit EventLoop and the Gui are NOT Send/Sync: keep them pinned to the process
// main thread across the two FFI calls (R64RuntimeInit stores, R64RunEventLoop consumes).
thread_local! {
    static EVENT_LOOP: RefCell<Option<EventLoopWrapper>> = const { RefCell::new(None) };
    static GUI: RefCell<Option<Gui<'static>>> = const { RefCell::new(None) };
}

/// Set true once R64RuntimeInit has run: the game is driving the machine through the
/// libultra runtime. Consulted by the os_cont SI post, distinguishing pre- vs
/// post-R64RuntimeInit runtime state.
static RUNTIME_ACTIVE: AtomicBool = AtomicBool::new(false);

/// True when the libultra runtime owns frame pacing / input (post-`R64RuntimeInit`).
pub fn runtime_active() -> bool {
    RUNTIME_ACTIVE.load(Ordering::SeqCst)
}

/// C-callable form of `runtime_active()`. Consumed by the `os_cont.c` SI post to guard the
/// `R64EventPost(OS_EVENT_SI)` call so controller sampling falls back to the direct
/// GamepadManager path whenever the runtime isn't active.
#[no_mangle]
pub extern "C" fn R64RuntimeActive() -> bool {
    runtime_active()
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Guest threads wait for the renderer before running any guest code, so its startup (a cold Metal
/// shader compile can take seconds) never counts against a guest timer such as OoT's 3 s RCP
/// watchdog. Only `R64RuntimeInit` expects a renderer; without it the gate is open.
struct RendererGate {
    expected: AtomicBool,
    ready: Mutex<bool>,
    created: Condvar,
}

impl RendererGate {
    const fn new() -> Self {
        RendererGate {
            expected: AtomicBool::new(false),
            ready: Mutex::new(false),
            created: Condvar::new(),
        }
    }

    fn wait(&self) {
        if !self.expected.load(Ordering::SeqCst) {
            return;
        }
        let mut ready = self.ready.lock().unwrap();
        while !*ready {
            ready = self.created.wait(ready).unwrap();
        }
    }

    fn open(&self) {
        *self.ready.lock().unwrap() = true;
        self.created.notify_all();
    }
}

static RENDERER: RendererGate = RendererGate::new();

pub(crate) fn wait_for_renderer() {
    RENDERER.wait();
}

pub(crate) fn renderer_created() {
    RENDERER.open();
}

/// Set by R64RuntimeInit; lets gui.rs know the runtime owns the loop.
pub fn mark_runtime_active() {
    RUNTIME_ACTIVE.store(true, Ordering::SeqCst);
}

/// Set by the winit CloseRequested handler or a winit `PumpStatus::Exit`, or by the render
/// thread on a fatal construction/loop failure; observed by R64RunEventLoop's pump loop.
pub fn request_shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}
pub fn shutdown_requested() -> bool {
    SHUTDOWN.load(Ordering::SeqCst)
}

/// Graceful teardown, in order: detach and drop the audio runtime (so no Arie player or
/// callback outlives the process), stop and join the VI retrace clock, then flush EEPROM to
/// disk. Audio is torn down BEFORE the EEPROM flush. Guest host-threads stay parked — the
/// cooperative scheduler has no async preemption; process exit after R64RunEventLoop returns
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
pub extern "C" fn R64RuntimeInit() {
    crate::init();
    RENDERER.expected.store(true, Ordering::SeqCst);
    // Force the process-global scheduler to exist before any guest thread starts.
    let _ = sched::scheduler();
    mark_runtime_active();

    let wrapper = EventLoopWrapper::new();
    // Leak a GamepadManager so the Gui can hold a `&'static mut` across the two FFI calls
    // (created here, consumed in R64RunEventLoop). The manager lives for the whole process.
    let gpm: &'static mut GamepadManager = Box::leak(Box::new(GamepadManager::new()));
    let mut gui = Gui::new("reality64", Some(gpm)).expect("reality64: failed to create Gui");

    // Seed the controller snapshot on THIS (main) thread before any guest thread starts, so the
    // first `osContInit` on thread5 observes a plugged-in controller (bits) instead of the
    // default-empty snapshot. The `!Send` manager is only ever pumped here on the main thread.
    gui.sample_gamepads_into_snapshot();

    EVENT_LOOP.with(|slot| *slot.borrow_mut() = Some(wrapper));
    GUI.with(|slot| *slot.borrow_mut() = Some(gui));
    log::info!("reality64: runtime initialized");
}

/// Set the window title on the main thread after R64RuntimeInit, before the window opens.
#[no_mangle]
pub extern "C" fn R64SetWindowTitle(title: *const std::ffi::c_char) {
    let title = unsafe { std::ffi::CStr::from_ptr(title) }.to_string_lossy();
    GUI.with(|slot| {
        if let Some(gui) = slot.borrow_mut().as_mut() {
            gui.set_title(&title);
        }
    });
}

/// Pump winit + gamepad on the process MAIN thread; returns only once shutdown_requested() is
/// observed. Graphics consume + present run OFF this thread, on the dedicated render thread
/// spawned by `Gui::resumed` (via `crate::render::spawn`) the first time the window comes up;
/// guest threads submit gfx tasks straight to that render thread over the `RenderMsg` channel
/// (`ultra::rcp::submit_and_wait`), bypassing this pump entirely. `Gui`/`EventLoopWrapper` were
/// created on THIS thread in R64RuntimeInit.
#[no_mangle]
pub extern "C" fn R64RunEventLoop() {
    use winit::platform::pump_events::PumpStatus;

    let mut gui = GUI
        .with(|slot| slot.borrow_mut().take())
        .expect("reality64: R64RunEventLoop must run on the R64RuntimeInit thread");
    let mut wrapper = EVENT_LOOP
        .with(|slot| slot.borrow_mut().take())
        .expect("reality64: R64RunEventLoop must run on the R64RuntimeInit thread");

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
    fn renderer_gate_holds_guest_threads_only_when_a_renderer_is_expected() {
        let gate: &'static RendererGate = Box::leak(Box::new(RendererGate::new()));
        gate.wait();

        gate.expected.store(true, Ordering::SeqCst);
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            gate.wait();
            done.send(()).unwrap();
        });
        let pending = std::time::Duration::from_millis(50);
        assert!(finished.recv_timeout(pending).is_err());
        gate.open();
        finished
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
    }

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
