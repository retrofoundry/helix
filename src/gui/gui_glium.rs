use anyhow::Result;

use glium::Frame;
use glutin::{event_loop::EventLoop, GlRequest};
use imgui::{Context, FontSource, Ui};
use imgui_glium_renderer::Renderer;

use winit::event::{Event, WindowEvent};

use std::str;
use std::time::Duration;
use std::{ffi::CStr, result::Result::Ok, time::Instant};
use winit::platform::run_return::EventLoopExtRunReturn;

use crate::fast3d::graphics::GraphicsIntermediateDevice;
use crate::fast3d::rcp::RCP;
use crate::fast3d::rdp::OutputDimensions;
use crate::gamepad::manager::GamepadManager;

use super::renderer::glium_device::GliumGraphicsDevice;

// use super::renderer::opengl_device::OpenGLGraphicsDevice;

pub struct Gui<'render> {
    // window
    pub display: glium::Display,

    // render
    renderer: Renderer,
    platform: imgui_winit_support::WinitPlatform,

    // imgui
    imgui: Context,

    // ui state
    ui_state: UIState,

    // draw callbacks
    draw_menu_callback: Box<dyn Fn(&Ui) + 'static>,
    draw_windows_callback: Box<dyn Fn(&Ui) + 'static>,

    // gamepad
    keyboard_manager: Option<&'render mut GamepadManager>,

    // game renderer
    rcp: RCP,
    pub intermediate_graphics_device: GraphicsIntermediateDevice,
    graphics_device: GliumGraphicsDevice<'render>,
}

pub struct UIState {
    last_frame_time: Instant,
}

pub struct EventLoopWrapper {
    event_loop: EventLoop<()>,
}

impl<'render> Gui<'render> {
    pub fn new<D, W>(
        title: &str,
        event_loop_wrapper: &EventLoopWrapper,
        draw_menu: D,
        draw_windows: W,
        keyboard_manager: Option<&'render mut GamepadManager>,
    ) -> Result<Self>
    where
        D: Fn(&Ui) + 'static,
        W: Fn(&Ui) + 'static,
    {
        let (width, height) = (800, 600);

        // Create the window

        let build = glutin::window::WindowBuilder::new()
            .with_title(title)
            .with_inner_size(glutin::dpi::LogicalSize::new(width, height));

        let context = glutin::ContextBuilder::new()
            .with_depth_buffer(24)
            .with_double_buffer(Some(true))
            .with_gl(GlRequest::Latest)
            .with_vsync(true);

        let display = glium::Display::new(build, context, &event_loop_wrapper.event_loop)?;

        // Setup ImGui
        let mut imgui = Context::create();

        // Create the egui + winit platform
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            display.gl_window().window(),
            imgui_winit_support::HiDpiMode::Default,
        );

        // Setup Dear ImGui style
        imgui.set_ini_filename(None);

        // Setup fonts
        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        // Setup Renderer
        let renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display)?;

        // Initial UI state
        let last_frame_time = Instant::now();

        Ok(Self {
            display,
            renderer,
            platform,
            imgui,
            ui_state: UIState { last_frame_time },
            draw_menu_callback: Box::new(draw_menu),
            draw_windows_callback: Box::new(draw_windows),
            keyboard_manager,
            rcp: RCP::new(),
            intermediate_graphics_device: GraphicsIntermediateDevice::new(),
            graphics_device: GliumGraphicsDevice::new(),
        })
    }

    fn handle_events(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        event_loop_wrapper
            .event_loop
            .run_return(|event, _, control_flow| match event {
                Event::MainEventsCleared => control_flow.set_exit(),
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    std::process::exit(0);
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let gl_window = self.display.gl_window();
                    gl_window.resize(size);
                }
                Event::WindowEvent {
                    event: WindowEvent::ModifiersChanged(modifiers),
                    ..
                } => {
                    if let Some(keyboard_manager) = self.keyboard_manager.as_mut() {
                        keyboard_manager.handle_modifiers_changed(modifiers);
                    }

                    // Forward the event over to Dear ImGui
                    let gl_window = self.display.gl_window();
                    self.platform
                        .handle_event(self.imgui.io_mut(), gl_window.window(), &event);
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { input, .. },
                    ..
                } => {
                    if let Some(keyboard_manager) = self.keyboard_manager.as_mut() {
                        keyboard_manager.handle_keyboard_input(input);
                    }

                    // Forward the event over to Dear ImGui
                    let gl_window = self.display.gl_window();
                    self.platform
                        .handle_event(self.imgui.io_mut(), gl_window.window(), &event);
                }
                event => {
                    let gl_window = self.display.gl_window();

                    self.platform
                        .handle_event(self.imgui.io_mut(), gl_window.window(), &event);
                }
            });
    }

    fn sync_frame_rate(&mut self) {
        const FRAME_INTERVAL_MS: u64 = 1000 / 30;

        let frame_duration = self.ui_state.last_frame_time.elapsed();
        if frame_duration < Duration::from_millis(FRAME_INTERVAL_MS) {
            let sleep_duration = Duration::from_millis(FRAME_INTERVAL_MS) - frame_duration;
            spin_sleep::sleep(sleep_duration);
        }

        let now = Instant::now();

        self.imgui
            .io_mut()
            .update_delta_time(now - self.ui_state.last_frame_time);

        self.ui_state.last_frame_time = now;
    }

    pub fn start_frame(&mut self, event_loop_wrapper: &mut EventLoopWrapper) -> Result<Frame> {
        // Handle events
        self.handle_events(event_loop_wrapper);

        // Grab current window size and store them
        let size = self.display.gl_window().window().inner_size();
        let dimensions = OutputDimensions {
            width: size.width,
            height: size.height,
            aspect_ratio: size.width as f32 / size.height as f32,
        };
        self.rcp.rdp.output_dimensions = dimensions;

        // Get the ImGui context and begin drawing the frame
        let gl_window = self.display.gl_window();
        self.platform
            .prepare_frame(self.imgui.io_mut(), gl_window.window())?;

        // Setup for drawing
        let target = self.display.draw();

        Ok(target)
    }

    fn render(&mut self, target: &mut Frame) -> Result<()> {
        // Begin drawing UI
        let ui = self.imgui.new_frame();
        ui.main_menu_bar(|| {
            (self.draw_menu_callback)(ui);
        });

        // Draw windows
        (self.draw_windows_callback)(ui);

        // Setup for drawing
        let gl_window = self.display.gl_window();

        // Render ImGui on top of any drawn content
        self.platform.prepare_render(ui, gl_window.window());
        let draw_data = self.imgui.render();

        self.renderer.render(target, draw_data)?;

        Ok(())
    }

    fn render_game(&mut self, target: &mut Frame) -> Result<()> {
        for draw_call in &self.intermediate_graphics_device.draw_calls {
            assert!(!draw_call.vbo.vbo.is_empty());

            self.graphics_device.set_cull_mode(draw_call.cull_mode);

            self.graphics_device
                .set_depth_stencil_params(draw_call.stencil);

            self.graphics_device.set_blend_state(draw_call.blend_state);
            self.graphics_device.set_viewport(&draw_call.viewport);
            self.graphics_device.set_scissor(draw_call.scissor);

            self.graphics_device.load_program(
                &self.display,
                draw_call.shader_hash,
                draw_call.other_mode_h,
                draw_call.other_mode_l,
                draw_call.geometry_mode,
                draw_call.combine,
            );

            // loop through textures and bind them
            for (index, hash) in draw_call.textures.iter().enumerate() {
                if let Some(hash) = hash {
                    let texture = self
                        .intermediate_graphics_device
                        .texture_cache
                        .get_mut(*hash)
                        .unwrap();
                    self.graphics_device
                        .bind_texture(&self.display, index, texture);
                }
            }

            // loop through samplers and bind them
            for (index, sampler) in draw_call.samplers.iter().enumerate() {
                if let Some(sampler) = sampler {
                    self.graphics_device.bind_sampler(index, sampler);
                }
            }

            // draw triangles
            self.graphics_device.draw_triangles(
                &self.display,
                target,
                draw_call.projection_matrix,
                &draw_call.fog,
                &draw_call.vbo.vbo,
                &draw_call.uniforms,
            );
        }

        Ok(())
    }

    pub fn create_event_loop() -> EventLoopWrapper {
        let event_loop = EventLoop::new();
        EventLoopWrapper { event_loop }
    }

    pub fn draw_lists(&mut self, mut frame: Frame, commands: usize) -> Result<()> {
        // Prepare the context device
        self.graphics_device.start_frame(&mut frame);

        // Run the RCP
        self.rcp
            .run(&mut self.intermediate_graphics_device, commands);
        self.render_game(&mut frame)?;

        // Finish rendering
        self.graphics_device.end_frame();

        // Render ImGui on top of any drawn content
        self.render(&mut frame)?;

        // Clear the draw calls
        self.intermediate_graphics_device.clear_draw_calls();

        // Swap buffers
        frame.finish()?;

        Ok(())
    }

    pub fn end_frame(&mut self) -> Result<()> {
        self.sync_frame_rate();
        Ok(())
    }
}

// MARK: - C API

type OnDraw = unsafe extern "C" fn(ui: &Ui);

#[no_mangle]
pub extern "C" fn GUICreateEventLoop() -> Box<EventLoopWrapper> {
    let event_loop = Gui::create_event_loop();
    Box::new(event_loop)
}

#[no_mangle]
pub unsafe extern "C" fn GUICreate<'render>(
    title_raw: *const i8,
    event_loop: Option<&'render mut EventLoopWrapper>,
    draw_menu: Option<OnDraw>,
    draw_windows: Option<OnDraw>,
    keyboard_manager: Option<&'render mut GamepadManager>,
) -> Box<Gui<'render>> {
    let title_str: &CStr = unsafe { CStr::from_ptr(title_raw) };
    let title: &str = str::from_utf8(title_str.to_bytes()).unwrap();

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
        keyboard_manager,
    )
    .unwrap();

    Box::new(gui)
}

#[no_mangle]
pub extern "C" fn GUIStartFrame(
    gui: Option<&mut Gui>,
    event_loop: Option<&mut EventLoopWrapper>,
) -> Box<Option<Frame>> {
    let gui = gui.unwrap();
    let event_loop = event_loop.unwrap();
    match gui.start_frame(event_loop) {
        Ok(frame) => Box::new(Some(frame)),
        Err(_) => Box::new(None),
    }
}

#[no_mangle]
pub extern "C" fn GUIDrawLists(gui: Option<&mut Gui>, frame: Option<Box<Frame>>, commands: u64) {
    let gui = gui.unwrap();
    let frame = frame.unwrap();
    gui.draw_lists(*frame, commands.try_into().unwrap())
        .unwrap();
}

#[no_mangle]
pub extern "C" fn GUIEndFrame(gui: Option<&mut Gui>) {
    let gui = gui.unwrap();
    gui.end_frame().unwrap();
}

#[no_mangle]
pub extern "C" fn GUIGetAspectRatio(gui: Option<&mut Gui>) -> f32 {
    let gui = gui.unwrap();
    gui.rcp.rdp.output_dimensions.aspect_ratio
}
