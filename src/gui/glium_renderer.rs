use crate::gui::{EventLoopWrapper, Frame};
use fast3d::output::RCPOutput;
use fast3d::rdp::OutputDimensions;
use fast3d_glium_renderer::glium_device::GliumGraphicsDevice;

pub struct Renderer<'a> {
    display: glium::Display,
    renderer: imgui_glium_renderer::Renderer,
    graphics_device: GliumGraphicsDevice<'a>,
}

impl<'a> Renderer<'a> {
    pub fn new(
        width: i32,
        height: i32,
        title: &str,
        event_loop_wrapper: &EventLoopWrapper,
        imgui: &mut imgui::Context,
    ) -> anyhow::Result<Self> {
        // Create the window
        let build = glutin::window::WindowBuilder::new()
            .with_title(title)
            .with_inner_size(glutin::dpi::LogicalSize::new(width, height));

        let context = glutin::ContextBuilder::new()
            .with_depth_buffer(24)
            .with_gl(glutin::GlRequest::Latest)
            .with_vsync(true);

        let display = glium::Display::new(build, context, &event_loop_wrapper.event_loop)?;

        // Create the renderer
        let renderer = imgui_glium_renderer::Renderer::init(imgui, &display)?;

        // Create graphics device
        let size = display.gl_window().window().inner_size();
        let graphics_device = GliumGraphicsDevice::new([size.width, size.height]);

        Ok(Self {
            display,
            renderer,
            graphics_device,
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
            self.display.gl_window().window(),
            imgui_winit_support::HiDpiMode::Default,
        );
    }

    pub fn handle_event<T>(
        &mut self,
        platform: &mut imgui_winit_support::WinitPlatform,
        imgui: &mut imgui::Context,
        event: &winit::event::Event<T>,
    ) {
        platform.handle_event(imgui.io_mut(), self.display.gl_window().window(), event);
    }

    pub fn prepare_frame(
        &self,
        platform: &mut imgui_winit_support::WinitPlatform,
        imgui: &mut imgui::Context,
    ) -> anyhow::Result<()> {
        platform.prepare_frame(imgui.io_mut(), self.display.gl_window().window())?;
        Ok(())
    }

    pub fn prepare_render(
        &self,
        platform: &mut imgui_winit_support::WinitPlatform,
        ui: &mut imgui::Ui,
    ) {
        platform.prepare_render(ui, self.display.gl_window().window());
    }

    // Rendering Functions

    pub fn content_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.display.gl_window().window().inner_size()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // there's a bug where at first the size is u32::MAX so we just ignore it
        if width == u32::MAX || height == u32::MAX {
            return;
        }

        log::trace!("Resizing to {:?}x{:?}", width, height);
        self.display
            .gl_window()
            .resize(glutin::dpi::PhysicalSize::new(width, height));
        self.graphics_device.resize([width, height]);
    }

    pub fn get_current_texture(&self) -> Option<Frame> {
        let frame = self.display.draw();
        Some(frame)
    }

    pub fn draw_content(
        &mut self,
        frame: &mut Frame,
        rcp_output: &mut RCPOutput,
        imgui_draw_data: &imgui::DrawData,
    ) -> anyhow::Result<()> {
        // Prepare the context device
        self.graphics_device.start_frame(frame);

        // Process the RCP output
        self.render_game(frame, rcp_output)?;

        // Render the ImGui content
        self.renderer.render(frame, imgui_draw_data)?;

        Ok(())
    }

    pub fn finish_render(&mut self, frame: Frame) -> anyhow::Result<()> {
        frame.finish()?;
        Ok(())
    }

    // MARK: - Helpers

    fn render_game(&mut self, frame: &mut Frame, rcp_output: &mut RCPOutput) -> anyhow::Result<()> {
        // omit the last draw call, because we know we that's an extra from the last flush
        // for draw_call in &self.rcp_output.draw_calls[..self.rcp_output.draw_calls.len() - 1] {
        for draw_call in rcp_output
            .draw_calls
            .iter()
            .take(rcp_output.draw_calls.len() - 1)
        {
            assert!(!draw_call.vbo.vbo.is_empty());

            self.graphics_device.set_cull_mode(draw_call.cull_mode);

            self.graphics_device
                .set_depth_stencil_params(draw_call.stencil);

            self.graphics_device.set_blend_state(draw_call.blend_state);
            self.graphics_device.set_viewport(&draw_call.viewport);
            self.graphics_device.set_scissor(draw_call.scissor);

            self.graphics_device.select_program(
                &self.display,
                draw_call.shader_id,
                draw_call.shader_config,
            );

            // loop through textures and bind them
            for (index, hash) in draw_call.texture_indices.iter().enumerate() {
                if let Some(hash) = hash {
                    let texture = rcp_output.texture_cache.get_mut(*hash).unwrap();
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
                frame,
                draw_call.projection_matrix,
                &draw_call.fog,
                &draw_call.vbo.vbo,
                draw_call.vbo.num_tris,
                &draw_call.uniforms,
            );
        }

        Ok(())
    }
}
