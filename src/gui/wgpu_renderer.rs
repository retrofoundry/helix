use crate::gui::{EventLoopWrapper, Frame};
use fast3d::RCPOutputCollector;

use fast3d_wgpu_renderer::WgpuRenderer;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn create_depth_texture(
    config: &wgpu::SurfaceConfiguration,
    device: &wgpu::Device,
) -> wgpu::TextureView {
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    });

    depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

pub struct Renderer<'a> {
    window: winit::window::Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    depth_texture: wgpu::TextureView,
    renderer: imgui_wgpu::Renderer,
    fast3d_renderer: WgpuRenderer<'a>,
}

impl<'a> Renderer<'a> {
    pub fn new(
        width: i32,
        height: i32,
        title: &str,
        event_loop_wrapper: &EventLoopWrapper,
        imgui: &mut imgui::Context,
    ) -> anyhow::Result<Self> {
        // Setup WGPU instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Create the window
        let (window, size, surface) = {
            let window = winit::window::WindowBuilder::new()
                .with_title(title)
                .with_inner_size(winit::dpi::LogicalSize::new(width, height))
                .with_resizable(true)
                .build(&event_loop_wrapper.event_loop)?;

            let size = window.inner_size();

            let surface = unsafe { instance.create_surface(&window) }?;

            (window, size, surface)
        };

        // Create the WGPU adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or(anyhow::anyhow!("Failed to find an appropriate adapter"))?;

        // Create the WGPU device
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ))?;

        // Create the swapchain
        let mut surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or(anyhow::anyhow!("Failed to get default surface config"))?;
        surface_config.format = wgpu::TextureFormat::Bgra8Unorm;

        surface.configure(&device, &surface_config);

        // Create the depth texture
        let depth_texture = create_depth_texture(&surface_config, &device);

        // Create Renderer
        let renderer_config = imgui_wgpu::RendererConfig {
            texture_format: surface_config.format,
            fragment_shader_entry_point: Some("fs_main_srgb"),
            ..Default::default()
        };

        let renderer = imgui_wgpu::Renderer::new(imgui, &device, &queue, renderer_config);

        // Create graphics device
        let fast3d_renderer = WgpuRenderer::new(&device, [size.width, size.height]);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            depth_texture,
            renderer,
            fast3d_renderer,
        })
    }

    // Platform Functions

    pub fn attach_window(
        &self,
        platform: &mut imgui_winit_support::WinitPlatform,
        imgui: &mut imgui::Context,
    ) {
        platform.attach_window(
            imgui.io_mut(),
            &self.window,
            imgui_winit_support::HiDpiMode::Default,
        );
    }

    pub fn handle_event<T>(
        &mut self,
        platform: &mut imgui_winit_support::WinitPlatform,
        imgui: &mut imgui::Context,
        event: &winit::event::Event<T>,
    ) {
        platform.handle_event(imgui.io_mut(), &self.window, event);
    }

    pub fn prepare_frame(
        &self,
        platform: &mut imgui_winit_support::WinitPlatform,
        imgui: &mut imgui::Context,
    ) -> anyhow::Result<()> {
        platform.prepare_frame(imgui.io_mut(), &self.window)?;
        Ok(())
    }

    pub fn prepare_render(
        &self,
        platform: &mut imgui_winit_support::WinitPlatform,
        ui: &mut imgui::Ui,
    ) {
        platform.prepare_render(ui, &self.window);
    }

    // Rendering Functions

    pub fn content_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.window.inner_size()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // there's a bug where at first the size is u32::MAX so we just ignore it
        if width == u32::MAX || height == u32::MAX {
            return;
        }

        log::trace!("Resizing to {:?}x{:?}", width, height);

        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
        self.depth_texture = create_depth_texture(&self.surface_config, &self.device);
        self.fast3d_renderer.resize([width, height]);
    }

    pub fn get_current_texture(&mut self) -> Option<Frame> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                log::trace!("Dropped frame due to error: {:?}", e);
                return None;
            }
        };

        Some(frame)
    }

    pub fn draw_content(
        &mut self,
        frame: &mut Frame,
        rcp_output_collector: &mut RCPOutputCollector,
        imgui_draw_data: &imgui::DrawData,
    ) -> anyhow::Result<()> {
        let frame_texture = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Prepare the context device
        self.fast3d_renderer.update_frame_count();

        // Process the RCP output
        self.fast3d_renderer.process_rcp_output(
            &self.device,
            &self.queue,
            self.surface_config.format,
            rcp_output_collector,
        );

        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Game Render Pass Command Encoder"),
                });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            // Draw the RCP output
            self.fast3d_renderer.draw(&mut rpass);
        }

        // Finish encoding and submit
        self.queue.submit(Some(encoder.finish()));

        // due to bug in macos or imgui-wgpu, we need to check for wrong texture size
        let fb_width = imgui_draw_data.display_size[0] * imgui_draw_data.framebuffer_scale[0];
        let fb_height = imgui_draw_data.display_size[1] * imgui_draw_data.framebuffer_scale[1];
        if fb_width as u32 == u32::MAX || fb_height as u32 == u32::MAX {
            return Ok(());
        }

        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ImGui Render Pass Command Encoder"),
                });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ImGui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            // Render the ImGui content
            self.renderer
                .render(imgui_draw_data, &self.queue, &self.device, &mut rpass)?;
        }

        // Finish encoding and submit
        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    pub fn finish_render(&mut self, frame: Frame) -> anyhow::Result<()> {
        frame.present();
        Ok(())
    }
}
