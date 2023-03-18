use anyhow::Result;
use imgui::{Condition, FontSource, MouseCursor, Image, Ui};
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use pollster::block_on;
#[cfg(feature = "cpp")]
use std::ffi::CStr;
#[cfg(feature = "cpp")]
use std::str;
use std::time::Instant;

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

    // draw callback
    draw_menu_callback: Box<dyn Fn(&Ui) + 'static>,
}

pub struct UIState {
    last_frame: Instant,
    last_cursor: Option<MouseCursor>,
    demo_open: bool,
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
            draw_menu_callback: Box::new(draw_menu),
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
        self.imgui_winit_platform
            .handle_event(self.imgui.io_mut(), &self.window, event);
    }

    pub fn draw(&mut self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        let now = Instant::now();
        self.imgui.io_mut().update_delta_time(now - self.ui_state.last_frame);
        self.ui_state.last_frame = now;
        
        self.imgui
            .io_mut()
            .update_delta_time(now - self.ui_state.last_frame);

        let frame = self.surface.get_current_texture()?;
        self.imgui_winit_platform.prepare_frame(self.imgui.io_mut(), &self.window)?;
        let ui = self.imgui.frame();

        {
            ui.main_menu_bar(|| {
                (self.draw_menu_callback)(&ui);
            });

            // let available_size = ui.content_region_avail();
            // Image::new(texture_id, available_size).build(ui);

            ui.show_metrics_window(&mut self.ui_state.demo_open);
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
            .render(self.imgui.render(), &self.queue, &self.device, &mut rpass)?;

        drop(rpass);

        self.queue.submit(Some(encoder.finish()));

        frame.present();

        Ok(())
    }
}

// static methods
impl Gui {
    pub fn create_event_loop() -> EventLoopWrapper {
        let event_loop = EventLoop::new();
        EventLoopWrapper { event_loop }
    }

    pub fn start(event_loop_wrapper: EventLoopWrapper, mut gui: Gui) {
        // let event_loop = EventLoop::new();
        // let mut gui = Gui::new(&event_loop).unwrap();

        event_loop_wrapper
            .event_loop
            .run(move |event, _, control_flow| {
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
                    Event::WindowEvent {
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

// MARK: - C API

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXGUICreateEventLoop() -> Box<EventLoopWrapper> {
    let event_loop = Gui::create_event_loop();
    Box::new(event_loop)
}

#[cfg(feature = "cpp")]
type DrawMenu = unsafe extern "C" fn();

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXGUICreate(
    title_raw: *const i8,
    event_loop: Option<&mut EventLoopWrapper>,
    draw_menu: Option<DrawMenu>,
) -> Box<Gui> {
    let title_str: &CStr = unsafe { CStr::from_ptr(title_raw) };
    let title: &str = str::from_utf8(title_str.to_bytes()).unwrap();

    let event_loop = event_loop.unwrap();
    let gui = Gui::new(title, event_loop, move |_ui| {
        unsafe {
            if let Some(draw_menu) = draw_menu {
                draw_menu();
            }
        }
    }).unwrap();

    Box::new(gui)
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXGUIStart(event_loop: Option<Box<EventLoopWrapper>>, gui: Option<Box<Gui>>) {
    let event_loop = event_loop.unwrap();
    let gui = gui.unwrap();
    Gui::start(*event_loop, *gui);
}
