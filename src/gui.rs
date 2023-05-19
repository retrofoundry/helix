use anyhow::Result;
use imgui::{Context, FontSource, Ui, MouseCursor, TextureId, Image};
use imgui_wgpu::{Renderer, RendererConfig, TextureConfig, Texture};
use imgui_winit_support::winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use log::trace;
use pollster::block_on;
use std::str;
use std::{ffi::CStr, time::Instant};
use winit::dpi::LogicalSize;
use winit::event_loop::ControlFlow;
use winit::platform::run_return::EventLoopExtRunReturn;

use crate::fast3d::{graphics::{GraphicsContext, dummy_device::DummyGraphicsDevice}, rcp::RCP};

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

    // content
    graphics_context: GraphicsContext,
    content_texture_id: TextureId,

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
        let mut last_frame_size = [width as f32, height as f32];

        // Create the window
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(LogicalSize::new(width, height))
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

        // Create the egui + winit platform
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
        let mut renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);

        // Initial UI state
        let last_frame_time = Instant::now();

        let mut last_cursor = None;

        // Setup graphics context
        let mut dummy_device = DummyGraphicsDevice::init(&surface_desc, &device, &queue);
        let graphics_context = GraphicsContext::new(Box::new(dummy_device));

        // Stores a texture for displaying with imgui::Image(),
        // also as a texture view for rendering into it

        let texture_config = TextureConfig {
            size: wgpu::Extent3d {
                width: width,
                height: height,
                ..Default::default()
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        };

        let texture = Texture::new(&device, &renderer, texture_config);
        let content_texture_id = renderer.textures.insert(texture);

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
            graphics_context,
            content_texture_id,
            ui_state: UIState { last_frame_time, last_cursor, last_frame_size },
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
                        std::process::exit(0);
                    }
                    Event::MainEventsCleared => control_flow.set_exit(),
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
        // Get the output frame
        let frame = self.surface.get_current_texture()?;

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Begin drawing UI
        let ui = self.imgui.frame();
        ui.main_menu_bar(|| {
            (self.draw_menu_callback)(ui);
        });

        // Draw the content

        let dummy_device = self.graphics_context
            .api
            .as_any_mut()
            .downcast_mut::<DummyGraphicsDevice>()
            .unwrap();

        // Render example normally at background
        dummy_device.update(ui.io().delta_time);
        dummy_device.setup_camera(&self.queue, ui.io().display_size);
        dummy_device.render(&view, &self.device, &self.queue);

        let available_size = ui.content_region_avail();
        Image::new(self.content_texture_id, available_size).build(ui);

        // Resize render target
        if available_size != self.ui_state.last_frame_size && available_size[0] >= 1.0 && available_size[1] >= 1.0 {
            trace!("Resizing render target: {}x{}", available_size[0], available_size[1]);
            self.ui_state.last_frame_size = available_size;
            let scale = &ui.io().display_framebuffer_scale;
            let texture_config = TextureConfig {
                size: wgpu::Extent3d {
                    width: (available_size[0] * scale[0]) as u32,
                    height: (available_size[1] * scale[1]) as u32,
                    ..Default::default()
                },
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                ..Default::default()
            };
            self.renderer.textures.replace(
                self.content_texture_id,
                Texture::new(&self.device, &self.renderer, texture_config),
            );
        }

        // Only render example to example_texture if thw window is not collapsed
        dummy_device.setup_camera(&self.queue, available_size);
        dummy_device.render(
            self.renderer.textures.get(self.content_texture_id).unwrap().view(),
            &self.device,
            &self.queue,
        );

        // Demo window for now
        let mut opened = true;
        ui.show_metrics_window(&mut opened);

        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main Command Encoder"),
                });

        if self.ui_state.last_cursor != ui.mouse_cursor() {
            self.ui_state.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(ui, &self.window);
        }

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
