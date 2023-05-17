use anyhow::Result;
use imgui::{Context, FontSource, Ui};
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use log::trace;
use pollster::block_on;
use std::str;
use std::{ffi::CStr, time::Instant};
use winit::event_loop::ControlFlow;
use winit::platform::run_return::EventLoopExtRunReturn;

use crate::fast3d::{graphics::GraphicsContext, rcp::RCP};

pub struct Gui {
    // window
    width: u32,
    height: u32,
    pub window: Window,

    // wgpu
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,

    // render
    renderer: Renderer,
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
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(PhysicalSize::new(width, height))
            .with_resizable(true)
            .build(&event_loop_wrapper.event_loop)?;

        // Create the wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Create the wgpu surface
        let surface = unsafe { instance.create_surface(&window)? };

        let hidpi_factor = window.scale_factor();

        // Request the adapter
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or(anyhow::anyhow!("Failed to request the adapter."))?;

        // Request the device and queue
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;

        // Configure the surface
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
        let mut imgui = Context::create();

        // Create the egui + sdl2 platform
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );

        // Setup Dear ImGui style
        imgui.set_ini_filename(None);

        // Setup fonts
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

        // Create the egui render pass
        let renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);

        // Initial UI state
        let last_frame_time = Instant::now();

        Ok(Self {
            width,
            height,
            window,
            surface,
            device,
            queue,
            renderer,
            platform,
            imgui,
            ui_state: UIState { last_frame_time },
            draw_menu_callback: Box::new(draw_menu),
            rcp: RCP::new(),
        })
    }

    fn recreate_swapchain(&mut self) -> Result<()> {
        let size = self.window.inner_size();
        self.width = size.width;
        self.height = size.height;
        trace!("Recreating swapchain: {}x{}", size.width, size.height);

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

    fn handle_events(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        event_loop_wrapper
            .event_loop
            .run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Poll;

                match event {
                    Event::WindowEvent {
                        event: WindowEvent::Resized(_),
                        ..
                    } => {
                        self.recreate_swapchain().unwrap();
                    }
                    Event::WindowEvent {
                        event: WindowEvent::ScaleFactorChanged { .. },
                        ..
                    } => {
                        self.recreate_swapchain().unwrap();
                    }
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        // control_flow.set_exit();
                        std::process::exit(0);
                    }
                    Event::MainEventsCleared => control_flow.set_exit(),
                    Event::RedrawRequested(_window_id) => {}
                    _ => (),
                }

                self.platform
                    .handle_event(self.imgui.io_mut(), &self.window, &event);
            });
    }

    pub fn start_frame(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        // Handle events
        self.handle_events(event_loop_wrapper);

        // Update the time
        let now = Instant::now();
        self.imgui
            .io_mut()
            .update_delta_time(now - self.ui_state.last_frame_time);
        self.ui_state.last_frame_time = now;

        // Get the ImGui context and begin drawing the frame
        self.platform
            .prepare_frame(self.imgui.io_mut(), &self.window);
    }

    fn render(&mut self) -> Result<()> {
        // Begin drawing UI
        let ui = self.imgui.frame();
        ui.main_menu_bar(|| {
            (self.draw_menu_callback)(ui);
        });

        // TODO: Draw game image here
        // let available_size = ui.content_region_avail();
        // Image::new(texture_id, available_size).build(ui);

        // Demo window for now
        let mut opened = true;
        ui.show_metrics_window(&mut opened);

        // Get the output frame
        let frame = self.surface.get_current_texture()?;

        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main Command Encoder"),
                });

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

    pub fn create_event_loop() -> EventLoopWrapper {
        let event_loop = EventLoop::new();
        EventLoopWrapper { event_loop }
    }

    pub fn draw_lists(&mut self, gfx_context: &GraphicsContext, commands: usize) -> Result<()> {
        self.rcp.run(gfx_context, commands);
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
pub extern "C" fn GUIDrawListsDummy(gui: Option<&mut Gui>) {
    let gui = gui.unwrap();
    gui.draw_lists_dummy().unwrap();
}

#[no_mangle]
pub extern "C" fn GUIEndFrame(gui: Option<&mut Gui>) {
    let gui = gui.unwrap();
    gui.end_frame();
}
