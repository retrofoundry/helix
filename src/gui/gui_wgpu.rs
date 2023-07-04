use anyhow::Result;
use imgui::{Context, FontSource, MouseCursor, Ui};
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use log::trace;
use wgpu::{SurfaceConfiguration, SurfaceTexture};

use std::{
    ffi::CStr,
    result::Result::Ok,
    str,
    time::{Duration, Instant},
};
use winit::{dpi::LogicalSize, platform::run_return::EventLoopExtRunReturn};

use crate::gamepad::manager::GamepadManager;
use fast3d::{output::RCPOutput, rcp::RCP, rdp::OutputDimensions};

use fast3d_wgpu_renderer::wgpu_device::WgpuGraphicsDevice;

pub struct Gui<'a> {
    // window
    pub window: Window,

    // render
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: SurfaceConfiguration,
    renderer: Renderer,
    platform: imgui_winit_support::WinitPlatform,

    // imgui
    imgui: Context,

    // ui state
    ui_state: UIState,

    // draw callbacks
    draw_menu_callback: Box<dyn Fn(&Ui) + 'static>,
    draw_windows_callback: Box<dyn Fn(&Ui) + 'static>,

    // gamepad
    gamepad_manager: Option<&'a mut GamepadManager>,

    // game renderer
    rcp: RCP,
    pub rcp_output: RCPOutput,
    graphics_device: WgpuGraphicsDevice,
}

pub struct UIState {
    last_frame_time: Instant,
    last_cursor: Option<MouseCursor>,
}

pub struct EventLoopWrapper {
    event_loop: EventLoop<()>,
}

impl<'a> Gui<'a> {
    pub fn new<D, W>(
        title: &str,
        event_loop_wrapper: &EventLoopWrapper,
        draw_menu: D,
        draw_windows: W,
        gamepad_manager: Option<&'a mut GamepadManager>,
    ) -> Result<Self>
    where
        D: Fn(&Ui) + 'static,
        W: Fn(&Ui) + 'static,
    {
        let (width, height) = (800, 600);

        // Setup WGPU instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Create the window
        let (window, size, surface) = {
            let window = WindowBuilder::new()
                .with_title(title)
                .with_inner_size(LogicalSize::new(width, height))
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

        // Setup ImGui
        let mut imgui = Context::create();
        imgui.io_mut().config_flags |=
            imgui::ConfigFlags::VIEWPORTS_ENABLE | imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE;

        // Create the imgui + winit platform
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );

        // Setup Dear ImGui style
        imgui.set_ini_filename(None);

        // Setup fonts
        let hidpi_factor = window.scale_factor();
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

        // Setup Renderer
        let renderer_config = RendererConfig {
            texture_format: surface_config.format,
            ..Default::default()
        };

        let renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);

        // Create graphics device
        let graphics_device = WgpuGraphicsDevice::new(&surface_config, &device);

        // Initial UI state
        let last_frame_time = Instant::now();
        let last_cursor = None;

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            renderer,
            platform,
            imgui,
            ui_state: UIState {
                last_frame_time,
                last_cursor,
            },
            draw_menu_callback: Box::new(draw_menu),
            draw_windows_callback: Box::new(draw_windows),
            gamepad_manager,
            rcp: RCP::default(),
            rcp_output: RCPOutput::default(),
            graphics_device,
        })
    }

    fn handle_events(&mut self, event_loop_wrapper: &mut EventLoopWrapper) {
        event_loop_wrapper
            .event_loop
            .run_return(|event, _, control_flow| {
                control_flow.set_poll();

                match event {
                    Event::WindowEvent {
                        event:
                            WindowEvent::Resized(size)
                            | WindowEvent::ScaleFactorChanged {
                                new_inner_size: &mut size,
                                ..
                            },
                        ..
                    } => {
                        // there's a bug where at first the size is u32::MAX so we just ignore it
                        if size.width == u32::MAX || size.height == u32::MAX {
                            return;
                        }

                        trace!("Resizing to {:?}", size);
                        self.surface_config.width = size.width.max(1);
                        self.surface_config.height = size.height.max(1);
                        self.surface.configure(&self.device, &self.surface_config);
                        self.graphics_device
                            .resize(&self.surface_config, &self.device);
                    }
                    Event::WindowEvent {
                        event: WindowEvent::ModifiersChanged(modifiers),
                        ..
                    } => {
                        if let Some(gamepad_manager) = self.gamepad_manager.as_mut() {
                            gamepad_manager.handle_modifiers_changed(modifiers);
                        }
                    }
                    Event::WindowEvent {
                        event: WindowEvent::KeyboardInput { input, .. },
                        ..
                    } => {
                        if let Some(gamepad_manager) = self.gamepad_manager.as_mut() {
                            gamepad_manager.handle_keyboard_input(input);
                        }
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

    fn sync_frame_rate(&mut self) {
        const FRAME_INTERVAL_MS: u64 = 1000 / 30;

        let frame_duration = self.ui_state.last_frame_time.elapsed();
        if frame_duration < Duration::from_millis(FRAME_INTERVAL_MS) {
            let sleep_duration = Duration::from_millis(FRAME_INTERVAL_MS) - frame_duration;
            spin_sleep::sleep(sleep_duration);
        }

        let now = Instant::now();

        self.imgui
            .io_mut()
            .update_delta_time(now - self.ui_state.last_frame_time);

        self.ui_state.last_frame_time = now;
    }

    pub fn start_frame(
        &mut self,
        event_loop_wrapper: &mut EventLoopWrapper,
    ) -> Result<Option<SurfaceTexture>> {
        // Handle events
        self.handle_events(event_loop_wrapper);

        // Start the frame
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                trace!("Dropped frame due to error: {:?}", e);
                return Ok(None);
            }
        };

        self.platform
            .prepare_frame(self.imgui.io_mut(), &self.window)?;

        Ok(Some(frame))
    }

    pub fn create_event_loop() -> EventLoopWrapper {
        let event_loop = EventLoop::new();
        EventLoopWrapper { event_loop }
    }

    pub fn draw_lists(&mut self, frame: SurfaceTexture, commands: usize) -> Result<()> {
        // Start frame
        let ui = self.imgui.new_frame();

        // Draw client menu bar
        ui.main_menu_bar(|| {
            (self.draw_menu_callback)(ui);
        });

        // Set RDP output dimensions
        let size = self.window.inner_size();
        let dimensions = OutputDimensions {
            width: size.width,
            height: size.height,
            aspect_ratio: size.width as f32 / size.height as f32,
        };
        self.rcp.rdp.output_dimensions = dimensions;

        // Texture we'll be drawing game and ImGui to
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            // Prepare the context device
            self.graphics_device.update_frame_count();

            // Run the RCP
            self.rcp.run(&mut self.rcp_output, commands);

            // Setup encoder that the RDP will use
            let mut encoder: wgpu::CommandEncoder =
                self.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Game Draw Command Encoder"),
                    });

            // Draw the RCP output
            // omit the last draw call, because we know we that's an extra from the last flush
            for (index, draw_call) in self
                .rcp_output
                .draw_calls
                .iter()
                .take(self.rcp_output.draw_calls.len() - 1)
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
                        let texture = self.rcp_output.texture_cache.get_mut(*hash).unwrap();
                        self.graphics_device.bind_texture(
                            &self.device,
                            &self.queue,
                            index,
                            texture,
                        );
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
                    &view,
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    &pipeline,
                    &texture_bind_group_layout,
                    &draw_call.viewport,
                    draw_call.scissor,
                    &draw_call.vbo.vbo,
                    draw_call.vbo.num_tris,
                );
            }

            // Draw client windows
            (self.draw_windows_callback)(ui);

            // Reset state
            self.rcp_output.clear_draw_calls();

            // Finish game encoding and submit
            self.queue.submit(Some(encoder.finish()));
        }

        // Draw ImGui to view
        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ImGui Command Encoder"),
                });

        if self.ui_state.last_cursor != ui.mouse_cursor() {
            self.ui_state.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(ui, &self.window);
        }

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ImGui Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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

    pub fn end_frame(&mut self) -> Result<()> {
        self.sync_frame_rate();
        Ok(())
    }
}

// MARK: - C API

type OnDraw = unsafe extern "C" fn(ui: &Ui);

#[no_mangle]
pub extern "C" fn GUICreateEventLoop() -> Box<EventLoopWrapper> {
    let event_loop = Gui::create_event_loop();
    Box::new(event_loop)
}

#[no_mangle]
pub unsafe extern "C" fn GUICreate<'a>(
    title_raw: *const i8,
    event_loop: Option<&'a mut EventLoopWrapper>,
    draw_menu: Option<OnDraw>,
    draw_windows: Option<OnDraw>,
    gamepad_manager: Option<&'a mut GamepadManager>,
) -> Box<Gui<'a>> {
    let title_str: &CStr = unsafe { CStr::from_ptr(title_raw) };
    let title: &str = str::from_utf8(title_str.to_bytes()).unwrap();

    let event_loop = event_loop.unwrap();
    let gui = Gui::new(
        title,
        event_loop,
        move |ui| unsafe {
            if let Some(draw_menu) = draw_menu {
                draw_menu(ui);
            }
        },
        move |ui| unsafe {
            if let Some(draw_windows) = draw_windows {
                draw_windows(ui);
            }
        },
        gamepad_manager,
    )
    .unwrap();

    Box::new(gui)
}

#[no_mangle]
pub extern "C" fn GUIStartFrame(
    gui: Option<&mut Gui>,
    event_loop: Option<&mut EventLoopWrapper>,
) -> Box<Option<SurfaceTexture>> {
    let gui = gui.unwrap();
    let event_loop = event_loop.unwrap();

    match gui.start_frame(event_loop) {
        Ok(frame) => Box::new(frame),
        Err(_) => Box::new(None),
    }
}

#[no_mangle]
pub extern "C" fn GUIDrawLists(
    gui: Option<&mut Gui>,
    current_frame: Box<Option<SurfaceTexture>>,
    commands: u64,
) {
    let gui = gui.unwrap();
    let current_frame = current_frame.unwrap();

    gui.draw_lists(current_frame, commands.try_into().unwrap())
        .unwrap();
}

#[no_mangle]
pub extern "C" fn GUIEndFrame(gui: Option<&mut Gui>) {
    let gui = gui.unwrap();
    gui.end_frame().unwrap();
}

#[no_mangle]
pub extern "C" fn GUIGetAspectRatio(gui: Option<&mut Gui>) -> f32 {
    let gui = gui.unwrap();
    gui.rcp.rdp.output_dimensions.aspect_ratio
}
