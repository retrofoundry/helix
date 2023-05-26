use anyhow::Result;
use glutin::{event_loop::EventLoop, Api, ContextWrapper, GlRequest, PossiblyCurrent};
use imgui::{Context, FontSource, MouseCursor, Ui};
use imgui_glow_renderer::{glow, AutoRenderer};
use imgui_winit_support::winit::window::Window;
use log::trace;
use std::str;
use std::{ffi::CStr, time::Instant};
use winit::event::{Event, WindowEvent};
use winit::platform::run_return::EventLoopExtRunReturn;

use crate::fast3d::{
    graphics::{dummy_device::DummyGraphicsDevice, GraphicsContext},
    rcp::RCP,
};

pub struct Gui {
    // window
    width: u32,
    height: u32,
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
}

pub struct UIState {
    last_frame_time: Instant,
    last_frame_size: [f32; 2],
    last_cursor: Option<MouseCursor>,
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
        let last_frame_size = [width as f32, height as f32];

        // Create the window

        let window = glutin::window::WindowBuilder::new()
            .with_title(title)
            .with_inner_size(glutin::dpi::LogicalSize::new(1024, 768));

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

        // Initial UI state
        let last_frame_time = Instant::now();

        let last_cursor = None;

        Ok(Self {
            width,
            height,
            window,
            renderer,
            platform,
            imgui,
            ui_state: UIState {
                last_frame_time,
                last_cursor,
                last_frame_size,
            },
            draw_menu_callback: Box::new(draw_menu),
            rcp: RCP::new(),
        })
    }

    // fn recreate_swapchain(&mut self) -> Result<()> {
    //     let size = self.window.inner_size();
    //     self.width = size.width;
    //     self.height = size.height;
    //     trace!("Recreating swapchain: {}x{}", size.width, size.height);

    //     let surface_desc = wgpu::SurfaceConfiguration {
    //         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    //         format: wgpu::TextureFormat::Bgra8UnormSrgb,
    //         width: size.width,
    //         height: size.height,
    //         present_mode: wgpu::PresentMode::Fifo,
    //         alpha_mode: wgpu::CompositeAlphaMode::Auto,
    //         view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
    //     };

    //     self.surface.configure(&self.device, &surface_desc);
    //     Ok(())
    // }

    fn handle_events(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        event_loop_wrapper
            .event_loop
            .run_return(|event, _, control_flow| {
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::Resized(_),
                        ..
                    } => {
                        trace!("Window resized");
                    }

                    glutin::event::Event::NewEvents(_) => {
                        let now = Instant::now();
                        self.imgui
                            .io_mut()
                            .update_delta_time(now.duration_since(self.ui_state.last_frame_time));
                        self.ui_state.last_frame_time = now;
                    }
                    glutin::event::Event::MainEventsCleared => control_flow.set_exit(),
                    glutin::event::Event::WindowEvent {
                        event: glutin::event::WindowEvent::CloseRequested,
                        ..
                    } => {
                        std::process::exit(0);
                    }
                    glutin::event::Event::LoopDestroyed => {
                        // let gl = ig_renderer.gl_context();
                        // tri_renderer.destroy(gl);
                    }
                    event => {
                        self.platform.handle_event(
                            self.imgui.io_mut(),
                            self.window.window(),
                            &event,
                        );
                    }
                }
            });
    }

    pub fn start_frame(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        // Handle events
        self.handle_events(event_loop_wrapper);

        // Update the time
        // let now = Instant::now();
        // self.imgui
        //     .io_mut()
        //     .update_delta_time(now - self.ui_state.last_frame_time);
        // self.ui_state.last_frame_time = now;

        // Get the ImGui context and begin drawing the frame
        self.platform
            .prepare_frame(self.imgui.io_mut(), self.window.window());
    }

    fn render(&mut self) -> Result<()> {
        // Begin drawing UI
        let ui = self.imgui.frame();
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

        // Swap buffers
        self.window.swap_buffers()?;

        Ok(())
    }

    pub fn create_event_loop() -> EventLoopWrapper {
        let event_loop = EventLoop::new();
        EventLoopWrapper { event_loop }
    }

    pub fn draw_lists(&mut self, gfx_context: &GraphicsContext, commands: usize) -> Result<()> {
        self.rcp
            .run(self.renderer.gl_context(), gfx_context, commands);
        // TODO: Draw rendered game image
        // let image = self.rcp.finish();

        self.render()?;

        Ok(())
    }

    pub fn draw_lists_dummy(&mut self) -> Result<()> {
        self.render()?;
        Ok(())
    }

    pub fn end_frame(&mut self) {}
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
pub extern "C" fn GUIDrawLists(
    gui: Option<&mut Gui>,
    gfx_context: Option<&mut GraphicsContext>,
    commands: u64,
) {
    let gui = gui.unwrap();
    let gfx_context = gfx_context.unwrap();
    gui.draw_lists(gfx_context, commands.try_into().unwrap())
        .unwrap();
}

#[no_mangle]
pub extern "C" fn GUIDrawListsDummy(
    gui: Option<&mut Gui>,
    gfx_context: Option<&mut GraphicsContext>,
) {
    let gui = gui.unwrap();
    let gfx_context = gfx_context.unwrap();

    let dummy_device = gfx_context
        .api
        .as_any_mut()
        .downcast_mut::<DummyGraphicsDevice>()
        .unwrap();

    dummy_device.render(gui.renderer.gl_context());

    gui.draw_lists_dummy().unwrap();
}

#[no_mangle]
pub extern "C" fn GUICreateGraphicsContext(gui: Option<&mut Gui>) -> Box<GraphicsContext> {
    let gui: &mut Gui = gui.unwrap();
    let dummy_device = DummyGraphicsDevice::new(gui.renderer.gl_context(), "#version 330");
    Box::new(GraphicsContext::new(Box::new(dummy_device)))
}

#[no_mangle]
pub extern "C" fn GUIEndFrame(gui: Option<&mut Gui>) {
    let gui = gui.unwrap();
    gui.end_frame();
}
