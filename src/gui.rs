use std::sync::Arc;

use crate::gamepad::manager::GamepadManager;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowAttributes, WindowId};

/// Wrapper around winit's event loop. The event loop lives separately from the
/// `Gui` so that the per-frame entry can hold a `&mut EventLoop` (for
/// `pump_app_events`) while passing the `Gui` as the `ApplicationHandler`.
pub struct EventLoopWrapper {
    event_loop: EventLoop<()>,
}

impl EventLoopWrapper {
    pub fn new() -> Self {
        Self {
            event_loop: EventLoop::new().expect("failed to create event loop"),
        }
    }
}

impl Default for EventLoopWrapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Main-thread half of the split: owns the winit window, pumps events, and drives the (`!Send`)
/// gamepad manager. On `resumed` it creates the wgpu surface and hands it off to the render
/// thread (`crate::render::spawn`), which owns `fast3d::Renderer` and does all consume/present
/// from then on — this struct never touches a `Renderer`.
pub struct Gui<'a> {
    title: String,
    width: u32,
    height: u32,
    window: Option<Arc<Window>>,
    render: Option<crate::render::RenderHandle>,
    gamepad_manager: Option<&'a mut GamepadManager>,
}

impl<'a> Gui<'a> {
    pub fn new(
        title: &str,
        gamepad_manager: Option<&'a mut GamepadManager>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            title: title.to_string(),
            width: 800,
            height: 600,
            window: None,
            render: None,
            gamepad_manager,
        })
    }

    /// Create the window (once), seed the widescreen aspect from its initial physical size, then
    /// create the wgpu surface and hand it off to a freshly spawned render thread.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title(&self.title)
                        .with_inner_size(LogicalSize::new(self.width, self.height))
                        .with_resizable(true),
                )
                .expect("create window"),
        );
        let size = window.inner_size();
        crate::render::set_aspect_from_size(size.width, size.height); // seed from initial physical size

        // `Arc<Window>` satisfies `Into<SurfaceTarget<'static>>` (carries its own display handle).
        let instance = wgpu::Instance::default(); // match fast3d Renderer::new exactly
        let surface = instance
            .create_surface(window.clone())
            .expect("create_surface on main");
        let handle = crate::render::spawn(crate::render::SurfaceHandoff {
            instance,
            surface,
            width: size.width.max(1),
            height: size.height.max(1),
        });

        self.window = Some(window);
        self.render = Some(handle);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                // Request graceful shutdown only; HLXRunEventLoop's pump loop observes it, wakes
                // the render thread with RenderMsg::Shutdown, joins it, then runs ultra::teardown()
                // (flushes EEPROM). No event_loop.exit() here — shutdown is centralized there.
                crate::ultra::request_shutdown();
            }
            WindowEvent::Resized(size) => {
                self.forward_resize(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = self.window.as_ref() {
                    let size = w.inner_size();
                    self.forward_resize(size.width, size.height);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                if let Some(gamepad_manager) = self.gamepad_manager.as_mut() {
                    gamepad_manager.handle_modifiers_changed(modifiers.state());
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(gamepad_manager) = self.gamepad_manager.as_mut() {
                    gamepad_manager.handle_keyboard_input(&event);
                }
            }
            _ => {}
        }
    }

    /// Update the main-fed widescreen aspect and forward the new physical size to the render
    /// thread for a surface reconfigure. Ignores winit's transient `u32::MAX` size and a
    /// minimized (zero-dimension) window.
    fn forward_resize(&self, width: u32, height: u32) {
        if crate::render::valid_surface_size(width, height).is_none() {
            return;
        }
        crate::render::set_aspect_from_size(width, height);
        crate::ultra::rcp::send_render_control(crate::ultra::rcp::RenderMsg::Resize {
            width,
            height,
        });
    }

    /// Take the render-thread handle so the caller can send `RenderMsg::Shutdown` and join it.
    /// `None` if the window (and thus the render thread) never came up.
    pub fn take_render_handle(&mut self) -> Option<crate::render::RenderHandle> {
        self.render.take()
    }

    pub fn renderer_name(&self) -> String {
        "WGPU".to_string()
    }

    /// Pump winit. Waits (blocking, no timeout) for `resumed()` to create the window on the very
    /// first pass, then pumps with a bounded ~2ms timeout so this thread never spins a core nor
    /// stalls guest threads for long. A winit `PumpStatus::Exit` at any point (including before
    /// the window exists) is returned to the caller so it can request shutdown.
    pub fn pump(&mut self, w: &mut EventLoopWrapper) -> PumpStatus {
        while self.window.is_none() {
            if let PumpStatus::Exit(code) = w.event_loop.pump_app_events(None, self) {
                return PumpStatus::Exit(code);
            }
        }
        w.event_loop
            .pump_app_events(Some(std::time::Duration::from_millis(2)), self)
    }

    /// Main-thread-only: pump the (`!Send`) gamepad manager and publish a `Send + Sync` snapshot
    /// for the libultra runtime's thread5 to read via `HLXControllerInit`/`HLXControllerRead`.
    /// The manager NEVER leaves this thread; only the plain snapshot crosses threads. Called each
    /// frame from `HLXRunEventLoop` (after `pump`, which drains winit keyboard input into the
    /// manager) and once during `HLXRuntimeInit` to seed the snapshot before threads start.
    pub fn sample_gamepads_into_snapshot(&mut self) {
        if let Some(manager) = self.gamepad_manager.as_mut() {
            let snapshot = manager.sample_snapshot();
            crate::gamepad::snapshot::publish(snapshot);
        }
    }
}

impl<'a> ApplicationHandler for Gui<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        Gui::resumed(self, event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        Gui::window_event(self, event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Frame pacing is driven by the decomp game loop, not winit.
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn GUICreateEventLoop() -> Box<EventLoopWrapper> {
    let event_loop = EventLoopWrapper::default();
    Box::new(event_loop)
}

#[no_mangle]
pub unsafe extern "C" fn GUICreate<'a>(
    title_raw: *const i8,
    _event_loop: Option<&'a mut EventLoopWrapper>,
    gamepad_manager: Option<&'a mut GamepadManager>,
) -> Box<Gui<'a>> {
    let title_str: &std::ffi::CStr = unsafe { std::ffi::CStr::from_ptr(title_raw) };
    let title: &str = std::str::from_utf8(title_str.to_bytes()).unwrap();

    let gui = Gui::new(title, gamepad_manager).unwrap();

    Box::new(gui)
}
