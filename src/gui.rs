use crate::gamepad::manager::GamepadManager;
use fast3d::output::RCPOutput;
use fast3d::rcp::RCP;
use fast3d::rdp::OutputDimensions;
use winit::platform::run_return::EventLoopExtRunReturn;

pub mod windows;

#[cfg(feature = "opengl_renderer")]
mod glium_renderer;
#[cfg(feature = "opengl_renderer")]
pub use glium_renderer::Renderer;
#[cfg(feature = "opengl_renderer")]
pub type Frame = glium::Frame;

#[cfg(feature = "wgpu_renderer")]
mod wgpu_renderer;
#[cfg(feature = "wgpu_renderer")]
pub use wgpu_renderer::Renderer;
#[cfg(feature = "wgpu_renderer")]
pub type Frame = wgpu::SurfaceTexture;

/// Represents the state of the UI.
pub struct UIState {
    last_frame_time: std::time::Instant,
    last_cursor: Option<imgui::MouseCursor>,
}

/// Wrapper around winit's event loop to allow for
/// the creation of the imgui context.
pub struct EventLoopWrapper {
    event_loop: winit::event_loop::EventLoop<()>,
}

impl EventLoopWrapper {
    pub fn new() -> Self {
        Self {
            event_loop: winit::event_loop::EventLoop::new(),
        }
    }
}

pub struct Gui<'a> {
    // imgui
    imgui: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,

    // ui state
    ui_state: UIState,

    // draw callbacks
    draw_menu_callback: Box<dyn Fn(&imgui::Ui) + 'a>,
    draw_windows_callback: Box<dyn Fn(&imgui::Ui) + 'a>,

    // gamepad
    gamepad_manager: Option<&'a mut GamepadManager>,

    // game renderer
    rcp: RCP,
    rcp_output: RCPOutput,
    gfx_renderer: Renderer<'a>,
}

impl<'a> Gui<'a> {
    pub fn new<D, W>(
        title: &str,
        event_loop_wrapper: &EventLoopWrapper,
        draw_menu: D,
        draw_windows: W,
        gamepad_manager: Option<&'a mut GamepadManager>,
    ) -> anyhow::Result<Self>
    where
        D: Fn(&imgui::Ui) + 'static,
        W: Fn(&imgui::Ui) + 'static,
    {
        // Setup ImGui
        let mut imgui = imgui::Context::create();

        // Create the imgui + winit platform
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);

        // Setup Dear ImGui style
        imgui.set_ini_filename(None);

        // Imgui Setup fonts
        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        // Setup Renderer
        let (width, height) = (800, 600);
        let renderer = Renderer::new(width, height, title, event_loop_wrapper, &mut imgui)?;
        renderer.attach_window(&mut platform, &mut imgui);

        // Initial UI state
        let last_frame_time = std::time::Instant::now();

        Ok(Self {
            imgui,
            platform,
            ui_state: UIState {
                last_frame_time,
                last_cursor: None,
            },
            draw_menu_callback: Box::new(draw_menu),
            draw_windows_callback: Box::new(draw_windows),
            gamepad_manager,
            rcp: RCP::new(),
            rcp_output: RCPOutput::new(),
            gfx_renderer: renderer,
        })
    }

    fn handle_events(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        event_loop_wrapper
            .event_loop
            .run_return(|event, _, control_flow| {
                match event {
                    winit::event::Event::MainEventsCleared => control_flow.set_exit(),
                    winit::event::Event::WindowEvent {
                        event: winit::event::WindowEvent::CloseRequested,
                        ..
                    } => std::process::exit(0),
                    winit::event::Event::WindowEvent {
                        event:
                            winit::event::WindowEvent::Resized(size)
                            | winit::event::WindowEvent::ScaleFactorChanged {
                                new_inner_size: &mut size,
                                ..
                            },
                        ..
                    } => {
                        self.gfx_renderer.resize(size.width, size.height);

                        // TODO: Fix resizing on OpenGL
                        #[cfg(feature = "wgpu_renderer")]
                        self.gfx_renderer
                            .handle_event(&mut self.platform, &mut self.imgui, &event);
                    }
                    winit::event::Event::WindowEvent {
                        event: winit::event::WindowEvent::ModifiersChanged(modifiers),
                        ..
                    } => {
                        if let Some(gamepad_manager) = self.gamepad_manager.as_mut() {
                            gamepad_manager.handle_modifiers_changed(modifiers);
                        }

                        self.gfx_renderer
                            .handle_event(&mut self.platform, &mut self.imgui, &event);
                    }
                    winit::event::Event::WindowEvent {
                        event: winit::event::WindowEvent::KeyboardInput { input, .. },
                        ..
                    } => {
                        if let Some(gamepad_manager) = self.gamepad_manager.as_mut() {
                            gamepad_manager.handle_keyboard_input(input);
                        }

                        self.gfx_renderer
                            .handle_event(&mut self.platform, &mut self.imgui, &event);
                    }
                    event => {
                        self.gfx_renderer
                            .handle_event(&mut self.platform, &mut self.imgui, &event)
                    }
                }
            });
    }

    fn sync_frame_rate(&mut self) {
        const FRAME_INTERVAL_MS: u64 = 1000 / 30;

        let frame_duration = self.ui_state.last_frame_time.elapsed();
        if frame_duration < std::time::Duration::from_millis(FRAME_INTERVAL_MS) {
            let sleep_duration =
                std::time::Duration::from_millis(FRAME_INTERVAL_MS) - frame_duration;
            spin_sleep::sleep(sleep_duration);
        }

        let now = std::time::Instant::now();

        self.imgui
            .io_mut()
            .update_delta_time(now - self.ui_state.last_frame_time);

        self.ui_state.last_frame_time = now;
    }

    pub fn start_frame(&mut self, event_loop_wrapper: &mut EventLoopWrapper) -> anyhow::Result<()> {
        // Handle events
        self.handle_events(event_loop_wrapper);

        // Prepare for drawing
        self.gfx_renderer
            .prepare_frame(&mut self.platform, &mut self.imgui)?;

        Ok(())
    }

    pub fn process_draw_lists(&mut self, commands: usize) -> anyhow::Result<()> {
        // Set RDP output dimensions
        let size = self.gfx_renderer.window_size();
        let dimensions = OutputDimensions {
            width: size.width,
            height: size.height,
            aspect_ratio: size.width as f32 / size.height as f32,
        };
        self.rcp.rdp.output_dimensions = dimensions;

        // Run the RCP
        self.rcp.run(&mut self.rcp_output, commands);

        // Grab the frame
        let mut frame = self.gfx_renderer.get_current_texture().unwrap();

        // Render RCP output
        self.gfx_renderer.process_rcp_output(
            &mut frame,
            &mut self.rcp_output,
            &self.rcp.rdp.output_dimensions,
        )?;

        // Render ImGui on top of any game content
        let ui = self.imgui.new_frame();
        ui.main_menu_bar(|| (self.draw_menu_callback)(ui));
        (self.draw_windows_callback)(ui);

        if self.ui_state.last_cursor != ui.mouse_cursor() {
            self.ui_state.last_cursor = ui.mouse_cursor();
            self.gfx_renderer.prepare_render(&mut self.platform, ui);
        }

        let draw_data = self.imgui.render();
        self.gfx_renderer
            .draw_imgui_content(&mut frame, draw_data)?;

        // Clear the draw calls
        self.rcp_output.clear_draw_calls();

        // Swap buffers
        self.gfx_renderer.finish_render(frame)?;

        Ok(())
    }

    pub fn end_frame(&mut self) {
        self.sync_frame_rate();
    }
}

// MARK: - C API

type OnDraw = unsafe extern "C" fn(ui: &imgui::Ui);

#[no_mangle]
pub extern "C" fn GUICreateEventLoop() -> Box<EventLoopWrapper> {
    let event_loop = EventLoopWrapper::new();
    Box::new(event_loop)
}

#[no_mangle]
pub unsafe extern "C" fn GUICreate<'a>(
    title_raw: *const i8,
    event_loop: Option<&'a mut EventLoopWrapper>,
    draw_menu: Option<OnDraw>,
    draw_windows: Option<OnDraw>,
    gamepad_manager: Option<&'a mut GamepadManager>,
) -> Box<Gui<'a>> {
    let title_str: &std::ffi::CStr = unsafe { std::ffi::CStr::from_ptr(title_raw) };
    let title: &str = std::str::from_utf8(title_str.to_bytes()).unwrap();

    let event_loop = event_loop.unwrap();
    let gui = Gui::new(
        title,
        event_loop,
        move |ui| unsafe {
            if let Some(draw_menu) = draw_menu {
                draw_menu(ui);
            }
        },
        move |ui| unsafe {
            if let Some(draw_windows) = draw_windows {
                draw_windows(ui);
            }
        },
        gamepad_manager,
    )
    .unwrap();

    Box::new(gui)
}

#[no_mangle]
pub extern "C" fn GUIStartFrame(gui: Option<&mut Gui>, event_loop: Option<&mut EventLoopWrapper>) {
    let gui = gui.unwrap();
    let event_loop = event_loop.unwrap();
    gui.start_frame(event_loop).unwrap();
}

#[no_mangle]
pub extern "C" fn GUIDrawLists(gui: Option<&mut Gui>, commands: u64) {
    let gui = gui.unwrap();
    gui.process_draw_lists(commands.try_into().unwrap())
        .unwrap();
}

#[no_mangle]
pub extern "C" fn GUIEndFrame(gui: Option<&mut Gui>) {
    let gui = gui.unwrap();
    gui.end_frame();
}

#[no_mangle]
pub extern "C" fn GUIGetAspectRatio(gui: Option<&mut Gui>) -> f32 {
    let gui = gui.unwrap();
    gui.rcp.rdp.output_dimensions.aspect_ratio
}
