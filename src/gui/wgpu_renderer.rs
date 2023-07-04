use crate::gui::{EventLoopWrapper, Frame};
use fast3d::output::RCPOutput;
use fast3d_wgpu_renderer::wgpu_device::WgpuGraphicsDevice;
use std::marker::PhantomData;

pub struct Renderer<'a> {
    window: winit::window::Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: imgui_wgpu::Renderer,
    graphics_device: WgpuGraphicsDevice,
    current_frame_texture: Option<wgpu::TextureView>,
    phantom: PhantomData<&'a ()>,
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

            let size = window.outer_size();

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

        let surface_view_format = surface_config.format.add_srgb_suffix();
        surface_config.view_formats.push(surface_view_format);

        surface.configure(&device, &surface_config);

        // Create Renderer
        let renderer_config = imgui_wgpu::RendererConfig {
            texture_format: surface_config.format,
            ..Default::default()
        };

        let renderer = imgui_wgpu::Renderer::new(imgui, &device, &queue, renderer_config);

        // Create graphics device
        let graphics_device = WgpuGraphicsDevice::new(&surface_config, &device);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            renderer,
            graphics_device,
            current_frame_texture: None,
            phantom: PhantomData,
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

    pub fn window_size(&self) -> winit::dpi::PhysicalSize<u32> {
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
        self.graphics_device
            .resize(&self.surface_config, &self.device);
    }

    pub fn get_current_texture(&mut self) -> Option<Frame> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                log::trace!("Dropped frame due to error: {:?}", e);
                return None;
            }
        };

        self.current_frame_texture = Some(
            frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        Some(frame)
    }

    pub fn process_rcp_output(
        &mut self,
        _frame: &mut Frame,
        rcp_output: &mut RCPOutput,
    ) -> anyhow::Result<()> {
        // Prepare the context device
        self.graphics_device.update_frame_count();

        // Setup encoder that the RDP will use
        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Game Draw Command Encoder"),
                });

        // Process the RCP output
        self.render_game(&mut encoder, rcp_output)?;

        // Finish game encoding and submit
        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    pub fn draw_imgui_content(
        &mut self,
        _frame: &mut Frame,
        draw_data: &imgui::DrawData,
    ) -> anyhow::Result<()> {
        // due to bug in macos or imgui-wgpu, we need to check for wrong texture size
        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if fb_width as u32 == u32::MAX || fb_height as u32 == u32::MAX {
            return Ok(());
        }

        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ImGui Command Encoder"),
                });

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ImGui Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.current_frame_texture.as_ref().unwrap(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        self.renderer
            .render(draw_data, &self.queue, &self.device, &mut rpass)?;

        drop(rpass);
        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    pub fn finish_render(&mut self, frame: Frame) -> anyhow::Result<()> {
        frame.present();
        self.current_frame_texture = None;
        Ok(())
    }

    // MARK: - Helpers

    fn render_game(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        rcp_output: &mut RCPOutput,
    ) -> anyhow::Result<()> {
        // omit the last draw call, because we know we that's an extra from the last flush
        // for draw_call in &self.rcp_output.draw_calls[..self.rcp_output.draw_calls.len() - 1] {
        for (index, draw_call) in rcp_output
            .draw_calls
            .iter()
            .take(rcp_output.draw_calls.len() - 1)
            .enumerate()
        {
            assert!(!draw_call.vbo.vbo.is_empty());

            self.graphics_device
                .update_current_height(draw_call.viewport.w as i32);

            self.graphics_device.select_program(
                &self.device,
                draw_call.shader_id,
                draw_call.shader_config,
            );

            // loop through textures and bind them
            for (index, hash) in draw_call.texture_indices.iter().enumerate() {
                if let Some(hash) = hash {
                    let texture = rcp_output.texture_cache.get_mut(*hash).unwrap();
                    self.graphics_device
                        .bind_texture(&self.device, &self.queue, index, texture);
                }
            }

            // loop through samplers and bind them
            for (index, sampler) in draw_call.samplers.iter().enumerate() {
                if let Some(sampler) = sampler {
                    self.graphics_device
                        .bind_sampler(&self.device, index, sampler);
                }
            }

            // set uniforms
            self.graphics_device.update_uniforms(
                &self.queue,
                draw_call.projection_matrix,
                &draw_call.fog,
                &draw_call.uniforms,
            );

            // create pipeline
            let (texture_bind_group_layout, pipeline) = self.graphics_device.create_pipeline(
                &self.device,
                self.surface_config.format,
                draw_call.blend_state,
                draw_call.cull_mode,
                draw_call.stencil,
            );

            // render triangles to texture
            self.graphics_device.draw_triangles(
                index,
                &self.current_frame_texture.as_ref().unwrap(),
                &self.device,
                &self.queue,
                encoder,
                &pipeline,
                &texture_bind_group_layout,
                &draw_call.viewport,
                draw_call.scissor,
                &draw_call.vbo.vbo,
                draw_call.vbo.num_tris,
            );
        }

        Ok(())
    }
}
