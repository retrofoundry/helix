use anyhow::Result;
use pollster::block_on;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct Gui {
    width: u32,
    height: u32,
    pub window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
}

impl Gui {
    pub fn start() {
        let event_loop = EventLoop::new();
        let mut gui = Gui::new(&event_loop).unwrap();

        event_loop.run(move |event, _, control_flow| {
            control_flow.set_wait();
            println!("{event:?}");

            match event {
                Event::WindowEvent { event, window_id } => {
                    // Update Egui integration so the UI works!
                    // let _response = gui.egui_integration.handle_event(&event);
                    match event {
                        WindowEvent::Resized(_) => {
                            gui.recreate_swapchain().unwrap();
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            gui.recreate_swapchain().unwrap();
                        }
                        WindowEvent::CloseRequested => {
                            if window_id == gui.window.id() {
                                control_flow.set_exit();
                            }
                        }
                        _ => (),
                    }
                }
                Event::MainEventsCleared => gui.window.request_redraw(),
                Event::RedrawRequested(_window_id) => gui.draw().unwrap(),
                _ => (),
            }
        });
    }

    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let (width, height) = (800, 600);
        let title = "Helix";

        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(PhysicalSize::new(width, height))
            .with_resizable(true)
            .build(event_loop)?;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }?;

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

        return Ok(Self {
            width,
            height,
            window,
            surface,
            device
        });
    }

    pub fn draw(&mut self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        Ok(())
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
}
