use anyhow::Result;
use imgui::{Condition, FontSource, MouseCursor};
use imgui_wgpu::{Renderer, RendererConfig};
use pollster::block_on;
use std::time::Instant;
use imgui_winit_support::winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct Gui {
    // window
    width: u32,
    height: u32,
    pub window: Window,
    
    // wgpu
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,

    // imgui
    imgui: imgui::Context,
    imgui_winit_platform: imgui_winit_support::WinitPlatform,

    // ui state
    ui_state: UIState,
}

pub struct UIState {
    last_frame: Instant,
    last_cursor: Option<MouseCursor>,
    demo_open: bool,
}

pub struct EventLoopWrapper {
    event_loop: EventLoop<()>,
}

// static methods
impl Gui {
    pub fn create_event_loop() -> EventLoopWrapper {
        let event_loop = EventLoop::new();
        return EventLoopWrapper { event_loop };
    }

    pub fn start(event_loop_wrapper: EventLoopWrapper, mut gui: Gui) {
        // let event_loop = EventLoop::new();
        // let mut gui = Gui::new(&event_loop).unwrap();

        event_loop_wrapper.event_loop.run(move |event, _, control_flow| {
            control_flow.set_wait();
            // println!("{event:?}");

            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    gui.recreate_swapchain().unwrap();
                }
                Event::WindowEvent {
                    event: WindowEvent::ScaleFactorChanged { .. },
                    ..
                } => {
                    gui.recreate_swapchain().unwrap();
                }
                | Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    control_flow.set_exit();
                }
                Event::MainEventsCleared => gui.window.request_redraw(),
                Event::RedrawRequested(_window_id) => gui.draw().unwrap(),
                _ => (),
            }

            gui.forward_event(&event);
        });
    }
}

impl Gui {
    pub fn new(event_loop_wrapper: &EventLoopWrapper) -> Result<Self> {
        let (width, height) = (800, 600);
        let title = "Helix";

        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(PhysicalSize::new(width, height))
            .with_resizable(true)
            .build(&event_loop_wrapper.event_loop)?;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }?;

        let hidpi_factor = window.scale_factor();

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).unwrap();

        let surface_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
        };

        surface.configure(&device, &surface_desc);

        // Setup ImGui
        let mut imgui = imgui::Context::create();
        let mut imgui_winit_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        imgui_winit_platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui.set_ini_filename(None);

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

        // Set up dear imgui wgpu renderer
        let renderer_config = RendererConfig {
            texture_format: surface_desc.format,
            ..Default::default()
        };

        let renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);

        // Initial UI state
        let last_frame = Instant::now();
        let demo_open = true;
        let last_cursor = None;

        Ok(Self {
            width,
            height,
            window,
            surface,
            device,
            queue,
            renderer,
            imgui,
            imgui_winit_platform,
            ui_state: UIState {
                last_frame,
                last_cursor,
                demo_open,
            },
        })
    }

    pub fn recreate_swapchain(&mut self) -> Result<()> {
        let size = self.window.inner_size();
        self.width = size.width;
        self.height = size.height;

        let surface_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
        };

        self.surface.configure(&self.device, &surface_desc);
        Ok(())
    }

    pub fn forward_event(&mut self, event: &Event<()>) {
        self.imgui_winit_platform.handle_event(self.imgui.io_mut(), &self.window, event);
    }

    pub fn draw(&mut self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        let delta_s = self.ui_state.last_frame.elapsed();
        let now = Instant::now();
        self.imgui
            .io_mut()
            .update_delta_time(now - self.ui_state.last_frame);

        let frame = self.surface.get_current_texture()?;
        self.imgui_winit_platform
            .prepare_frame(self.imgui.io_mut(), &self.window)
            .expect("Failed to prepare frame");
        let ui = self.imgui.frame();

        {
            let window = ui.window("Hello world");
            window
                .size([300.0, 100.0], Condition::FirstUseEver)
                .build(|| {
                    ui.text("Hello world!");
                    ui.text("This...is...imgui-rs on WGPU!");
                    ui.separator();
                    let mouse_pos = ui.io().mouse_pos;
                    ui.text(format!(
                        "Mouse Position: ({:.1},{:.1})",
                        mouse_pos[0], mouse_pos[1]
                    ));
                });

            let window = ui.window("Hello too");
            window
                .size([400.0, 200.0], Condition::FirstUseEver)
                .position([400.0, 200.0], Condition::FirstUseEver)
                .build(|| {
                    ui.text(format!("Frametime: {delta_s:?}"));
                });

            ui.show_demo_window(&mut self.ui_state.demo_open);
        }

        let mut encoder: wgpu::CommandEncoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if self.ui_state.last_cursor != ui.mouse_cursor() {
            self.ui_state.last_cursor = ui.mouse_cursor();
            self.imgui_winit_platform.prepare_render(ui, &self.window);
        }

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        self.renderer
            .render(self.imgui.render(), &self.queue, &self.device, &mut rpass)
            .expect("Rendering failed");

        drop(rpass);

        self.queue.submit(Some(encoder.finish()));

        frame.present();

        Ok(())
    }
}


// MARK: - C API

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXGUICreateEventLoop() -> Box<EventLoopWrapper> {
    let event_loop = Gui::create_event_loop();
    return Box::new(event_loop);
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXGUICreate(event_loop: Option<&mut EventLoopWrapper>) -> Box<Gui> {
    let event_loop = event_loop.unwrap();
    let gui = Gui::new(event_loop).unwrap();
    return Box::new(gui);
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXGUIStart(event_loop: Option<Box<EventLoopWrapper>>, gui: Option<Box<Gui>>) {
    let event_loop = event_loop.unwrap();
    let gui = gui.unwrap();
    Gui::start(*event_loop, *gui);
}

