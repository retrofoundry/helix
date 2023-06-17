use anyhow::{Ok, Result};
use glutin::{event_loop::EventLoop, Api, ContextWrapper, GlRequest, PossiblyCurrent};
use imgui::{Context, FontSource, Ui};
use imgui_glow_renderer::{glow, AutoRenderer};
use imgui_winit_support::winit::window::Window;

use std::str;
use std::time::Duration;
use std::{ffi::CStr, time::Instant};
use winit::platform::run_return::EventLoopExtRunReturn;

use crate::fast3d::graphics::GraphicsIntermediateDevice;
use crate::fast3d::rcp::RCP;
use crate::fast3d::rdp::OutputDimensions;

use super::renderer::opengl_device::OpenGLGraphicsDevice;

pub struct Gui {
    // window
    pub window: ContextWrapper<PossiblyCurrent, Window>,

    // render
    renderer: AutoRenderer,
    platform: imgui_winit_support::WinitPlatform,

    // imgui
    imgui: Context,

    // ui state
    ui_state: UIState,

    // draw callbacks
    draw_menu_callback: Box<dyn Fn(&Ui) + 'static>,

    // game renderer
    rcp: RCP,
    intermediate_graphics_device: GraphicsIntermediateDevice,
    graphics_device: OpenGLGraphicsDevice,
}

pub struct UIState {
    last_frame_time: Instant,
}

pub struct EventLoopWrapper {
    event_loop: EventLoop<()>,
}

impl Gui {
    pub fn new<D>(title: &str, event_loop_wrapper: &EventLoopWrapper, draw_menu: D) -> Result<Self>
    where
        D: Fn(&Ui) + 'static,
    {
        let (width, height) = (800, 600);

        // Create the window

        let window = glutin::window::WindowBuilder::new()
            .with_title(title)
            .with_inner_size(glutin::dpi::LogicalSize::new(width, height));

        let window = glutin::ContextBuilder::new()
            .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
            .with_vsync(true)
            .build_windowed(window, &event_loop_wrapper.event_loop)
            .expect("could not create window");

        let window = unsafe {
            window
                .make_current()
                .expect("could not make window context current")
        };

        // Setup ImGui
        let mut imgui = Context::create();

        // Create the egui + winit platform
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            window.window(),
            imgui_winit_support::HiDpiMode::Default,
        );

        // Setup Dear ImGui style
        imgui.set_ini_filename(None);

        // Setup fonts
        // let hidpi_factor = window.window().scale_factor();
        let font_size = (13.0 * platform.hidpi_factor()) as f32;
        imgui.io_mut().font_global_scale = (1.0 / platform.hidpi_factor()) as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        // Initialize gl
        let gl =
            unsafe { glow::Context::from_loader_function(|s| window.get_proc_address(s).cast()) };

        // Setup Renderer
        let renderer = imgui_glow_renderer::AutoRenderer::initialize(gl, &mut imgui)
            .expect("Failed to create renderer");

        // Initialize graphics device
        let graphics_device = OpenGLGraphicsDevice::new(renderer.gl_context());

        // Initial UI state
        let last_frame_time = Instant::now();

        Ok(Self {
            window,
            renderer,
            platform,
            imgui,
            ui_state: UIState { last_frame_time },
            draw_menu_callback: Box::new(draw_menu),
            rcp: RCP::new(),
            intermediate_graphics_device: GraphicsIntermediateDevice::new(),
            graphics_device,
        })
    }

    fn handle_events(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        event_loop_wrapper
            .event_loop
            .run_return(|event, _, control_flow| {
                match event {
                    glutin::event::Event::MainEventsCleared => control_flow.set_exit(),
                    glutin::event::Event::WindowEvent {
                        event: glutin::event::WindowEvent::CloseRequested,
                        ..
                    } => {
                        std::process::exit(0);
                    }
                    glutin::event::Event::WindowEvent {
                        event: glutin::event::WindowEvent::Resized(size),
                        ..
                    } => {
                        self.window.resize(size);
                    }
                    _ => (),
                }

                self.platform
                    .handle_event(self.imgui.io_mut(), self.window.window(), &event);
            });
    }

    fn sync_frame_rate(&mut self) {
        // TODO: Fix off by one error & test other OSes
        const FRAME_INTERVAL_MS: u64 = 1000 / (30 + 1) as u64;

        let frame_duration = self.ui_state.last_frame_time.elapsed();

        if frame_duration < Duration::from_millis(FRAME_INTERVAL_MS) {
            let sleep_duration = Duration::from_millis(FRAME_INTERVAL_MS) - frame_duration;
            spin_sleep::sleep(sleep_duration);
        }
    }

    pub fn start_frame(&mut self, event_loop_wrapper: &mut EventLoopWrapper) -> Result<()> {
        // Handle events
        self.handle_events(event_loop_wrapper);

        // Update delta time
        let now = Instant::now();
        self.imgui
            .io_mut()
            .update_delta_time(now - self.ui_state.last_frame_time);
        self.ui_state.last_frame_time = now;

        // Grab current window size and store them
        let size = self.window.window().inner_size();
        let dimensions = OutputDimensions {
            width: size.width,
            height: size.height,
            aspect_ratio: size.width as f32 / size.height as f32,
        };
        self.rcp.rdp.output_dimensions = dimensions;

        // Get the ImGui context and begin drawing the frame
        self.platform
            .prepare_frame(self.imgui.io_mut(), self.window.window())?;

        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        // Begin drawing UI
        let ui = self.imgui.new_frame();
        ui.main_menu_bar(|| {
            (self.draw_menu_callback)(ui);
        });

        // Demo window for now
        let mut opened = true;
        ui.show_metrics_window(&mut opened);

        self.platform.prepare_render(ui, self.window.window());
        let draw_data = self.imgui.render();

        // Render ImGui on top of any drawn content
        self.renderer
            .render(draw_data)
            .expect("error rendering imgui");

        Ok(())
    }

    fn render_game(&mut self) -> Result<()> {
        for draw_call in &self.intermediate_graphics_device.draw_calls {
            assert!(!draw_call.vbo.vbo.is_empty());

            let gl = self.renderer.gl_context();

            self.graphics_device.set_cull_mode(gl, draw_call.cull_mode);

            self.graphics_device
                .set_depth_stencil_params(gl, draw_call.stencil);

            self.graphics_device
                .set_blend_state(gl, draw_call.blend_state);
            self.graphics_device.set_viewport(gl, &draw_call.viewport);
            self.graphics_device.set_scissor(gl, draw_call.scissor);

            self.graphics_device.load_program(
                gl,
                draw_call.other_mode_h,
                draw_call.other_mode_l,
                draw_call.combine,
                draw_call.tile_descriptors,
            );

            // loop through textures and bind them
            for (index, hash) in draw_call.textures.iter().enumerate() {
                if let Some(hash) = hash {
                    let texture = self
                        .intermediate_graphics_device
                        .texture_cache
                        .get_mut(*hash)
                        .unwrap();
                    self.graphics_device.bind_texture(gl, index, texture);
                }
            }

            // loop through samplers and bind them
            for (index, sampler) in draw_call.samplers.iter().enumerate() {
                if let Some(sampler) = sampler {
                    self.graphics_device.bind_sampler(gl, index, sampler);
                }
            }

            // set uniforms
            self.graphics_device.set_uniforms(
                gl,
                &draw_call.uniforms.fog_color,
                &draw_call.uniforms.blend_color,
                &draw_call.uniforms.prim_color,
                &draw_call.uniforms.env_color,
                &draw_call.uniforms.key_center,
                &draw_call.uniforms.key_scale,
                &draw_call.uniforms.prim_lod,
                &draw_call.uniforms.convert_k,
            );

            self.graphics_device
                .draw_triangles(gl, &draw_call.vbo.vbo, draw_call.vbo.num_tris);
        }

        Ok(())
    }

    pub fn create_event_loop() -> EventLoopWrapper {
        let event_loop = EventLoop::new();
        EventLoopWrapper { event_loop }
    }

    pub fn draw_lists(&mut self, commands: usize) -> Result<()> {
        // Prepare the context device
        self.graphics_device.start_frame(self.renderer.gl_context());

        // Run the RCP
        self.rcp
            .run(&mut self.intermediate_graphics_device, commands);
        self.render_game()?;

        // Finish rendering
        self.graphics_device.end_frame();
        self.intermediate_graphics_device.clear_draw_calls();

        // Render ImGui on top of any drawn content
        self.render()?;

        // Swap buffers
        self.window.swap_buffers()?;

        Ok(())
    }

    pub fn end_frame(&mut self) -> Result<()> {
        self.sync_frame_rate();
        Ok(())
    }
}

// MARK: - C API

type OnDraw = unsafe extern "C" fn();

#[no_mangle]
pub extern "C" fn GUICreateEventLoop() -> Box<EventLoopWrapper> {
    let event_loop = Gui::create_event_loop();
    Box::new(event_loop)
}

#[no_mangle]
pub unsafe extern "C" fn GUICreate(
    title_raw: *const i8,
    event_loop: Option<&mut EventLoopWrapper>,
    draw_menu: Option<OnDraw>,
) -> Box<Gui> {
    let title_str: &CStr = unsafe { CStr::from_ptr(title_raw) };
    let title: &str = str::from_utf8(title_str.to_bytes()).unwrap();

    let event_loop = event_loop.unwrap();
    let gui = Gui::new(title, event_loop, move |_ui| unsafe {
        if let Some(draw_menu) = draw_menu {
            draw_menu();
        }
    })
    .unwrap();

    Box::new(gui)
}

#[no_mangle]
pub extern "C" fn GUIStartFrame(gui: Option<&mut Gui>, event_loop: Option<&mut EventLoopWrapper>) {
    let gui = gui.unwrap();
    let event_loop = event_loop.unwrap();
    gui.start_frame(event_loop);
}

#[no_mangle]
pub extern "C" fn GUIDrawLists(gui: Option<&mut Gui>, _frame: libc::c_void, commands: u64) {
    let gui = gui.unwrap();
    gui.draw_lists(commands.try_into().unwrap()).unwrap();
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
