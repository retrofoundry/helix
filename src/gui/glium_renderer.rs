use crate::gui::{EventLoopWrapper, Frame};
use fast3d::rdp::OutputDimensions;
use fast3d::RCPOutputCollector;
use fast3d_glium_renderer::GliumRenderer;

pub struct Renderer<'a> {
    display: glium::Display,
    renderer: imgui_glium_renderer::Renderer,
    fast3d_renderer: GliumRenderer<'a>,
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
        let fast3d_renderer = GliumRenderer::new([size.width, size.height]);

        Ok(Self {
            display,
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

    pub fn name(&self) -> String {
        format!("Glium | OpenGL")
    }

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
        self.fast3d_renderer.resize([width, height]);
    }

    pub fn get_current_texture(&self) -> Option<Frame> {
        let frame = self.display.draw();
        Some(frame)
    }

    pub fn draw_content(
        &mut self,
        frame: &mut Frame,
        rcp_output_collector: &mut RCPOutputCollector,
        imgui_draw_data: &imgui::DrawData,
    ) -> anyhow::Result<()> {
        // Prepare the context device
        self.fast3d_renderer.start_frame(frame);

        // Process the RCP output
        self.fast3d_renderer
            .render_rcp_output(rcp_output_collector, &self.display, frame);

        // Render the ImGui content
        self.renderer.render(frame, imgui_draw_data)?;

        Ok(())
    }

    pub fn finish_render(&mut self, frame: Frame) -> anyhow::Result<()> {
        frame.finish()?;
        Ok(())
    }
}
