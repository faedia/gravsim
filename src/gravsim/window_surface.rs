use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::{dpi::Size, event::WindowEvent, event_loop::ActiveEventLoop, window};

use crate::gravsim::{
    application::Application,
    shader::{FragmentShader, VertexShader},
};

pub struct WindowSurface<App: Application> {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<winit::window::Window>,
    imgui_context: imgui::Context,
    imgui_platform: imgui_winit_support::WinitPlatform,
    imgui_renderer: imgui_wgpu::Renderer,
    last_frame_time: std::time::Instant,
    app: Option<App>,
}

pub struct RenderContext<'a> {
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
}

pub struct RenderPassDesc {
    pub label: Option<&'static str>,
    pub clear_color: wgpu::Color,
}

impl<'a> RenderContext<'a> {
    pub fn render_pass(&mut self, desc: RenderPassDesc, f: impl FnOnce(&mut wgpu::RenderPass)) {
        let mut render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: desc.label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(desc.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        f(&mut render_pass);
    }
}

impl<App: Application> WindowSurface<App> {
    pub async fn new(event_loop: &ActiveEventLoop) -> Self {
        let start_time = std::time::Instant::now();

        let window = Self::create_window(event_loop);
        let (surface, device, queue, config) = Self::create_wgpu(window.clone()).await.unwrap();

        window.set_visible(true);
        window.focus_window();
        window
            .set_cursor_grab(window::CursorGrabMode::Confined)
            .ok();

        let mut context = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::new(&mut context);
        platform.attach_window(
            context.io_mut(),
            &*window,
            imgui_winit_support::HiDpiMode::Default,
        );
        context.set_ini_filename(None);
        let imgui_renderer = imgui_wgpu::Renderer::new(
            &mut context,
            &device,
            &queue,
            imgui_wgpu::RendererConfig {
                texture_format: config.format,
                ..Default::default()
            },
        );

        let mut tmp = Self {
            surface,
            device,
            queue,
            config,
            window,
            imgui_context: context,
            imgui_platform: platform,
            imgui_renderer,
            last_frame_time: std::time::Instant::now(),
            app: None,
        };

        tmp.app = Some(App::new(&mut tmp));

        log::info!(
            "Window and WGPU initialized in {:.2?}",
            start_time.elapsed()
        );

        tmp
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self) {
        if self.window.is_minimized().unwrap() {
            return;
        }

        let now = std::time::Instant::now();
        self.imgui_context
            .io_mut()
            .update_delta_time(now - self.last_frame_time);
        self.last_frame_time = now;

        self.window.request_redraw();

        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                self.resize(self.config.width, self.config.height);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture after resize")
            }
            Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture after reconfigure")
            }
            Err(e) => panic!("Failed to acquire next swap chain texture: {:?}", e),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let mut app = self.app.take().expect("App must be present");
        {
            self.imgui_platform
                .prepare_frame(self.imgui_context.io_mut(), &*self.window)
                .expect("Failed to prepare frame");
            let ui = self.imgui_context.frame();
            app.ui(ui);

            app.render(&mut RenderContext {
                encoder: &mut encoder,
                view: &view,
            });

            self.imgui_platform.prepare_render(ui, &*self.window);

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Imgui Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                self.imgui_renderer
                    .render(
                        self.imgui_context.render(),
                        &self.queue,
                        &self.device,
                        &mut rpass,
                    )
                    .expect("Rendering imgui failed");
            }
        }

        self.app = Some(app);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    pub fn handle_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) -> anyhow::Result<()> {
        match event {
            WindowEvent::CloseRequested => {
                log::trace!("Closing window {:?}", window_id);
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                log::trace!("Resizing window {:?} to {:?}", window_id, size);
                self.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                log::trace!("Redrawing window {:?}", window_id);
                self.render();
            }
            WindowEvent::Focused(focused) => {
                log::trace!("Window {:?} focused: {}", window_id, focused);
                if self.window.fullscreen().is_some() {
                    self.window.set_minimized(!focused);
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                ref event,
                is_synthetic,
            } => {
                log::trace!(
                    "Keyboard input on window {:?}: device_id={:?}, event={:?}, is_synthetic={}",
                    window_id,
                    device_id,
                    event,
                    is_synthetic
                );

                if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::F11)
                    && event.state == winit::event::ElementState::Pressed
                {
                    if self.window.fullscreen().is_some() {
                        self.window.set_fullscreen(None);
                        let _ = self.window.request_inner_size(Size::new(
                            winit::dpi::LogicalSize::new(1920.0, 1080.0),
                        ));
                        self.resize(1920, 1080);
                    } else {
                        if let Some(monitor) = event_loop.primary_monitor() {
                            let first_mode = monitor.video_modes().next();
                            if let Some(video_mode) = first_mode {
                                let _ = self.window.request_inner_size(Size::new(
                                    winit::dpi::PhysicalSize {
                                        width: video_mode.size().width,
                                        height: video_mode.size().height,
                                    },
                                ));

                                self.resize(video_mode.size().width, video_mode.size().height);

                                self.window.set_fullscreen(Some(
                                    winit::window::Fullscreen::Exclusive(video_mode),
                                ));
                            }
                        }
                    }
                }
            }
            _ => log::trace!("Skipping event {:?}", event),
        }

        self.imgui_platform.handle_event::<()>(
            self.imgui_context.io_mut(),
            &*self.window,
            &winit::event::Event::WindowEvent { window_id, event },
        );

        Ok(())
    }

    fn create_window(event_loop: &ActiveEventLoop) -> Arc<winit::window::Window> {
        log::info!("Creating the window");

        let mut window_attributes = winit::window::Window::default_attributes();
        window_attributes.title = "GravSim".into();

        if let Some(monitor) = event_loop.primary_monitor() {
            log::info!("Using primary monitor: {:?}", monitor.name());

            let first_mode = monitor.video_modes().next();
            if let Some(video_mode) = first_mode {
                log::info!(
                    "Setting fullscreen with video mode: {}x{} @ {} mHz ({} bpp)",
                    video_mode.size().width,
                    video_mode.size().height,
                    video_mode.refresh_rate_millihertz(),
                    video_mode.bit_depth()
                );

                window_attributes.inner_size = Some(Size::new(winit::dpi::PhysicalSize {
                    width: video_mode.size().width,
                    height: video_mode.size().height,
                }));
                window_attributes.fullscreen = Some(window::Fullscreen::Exclusive(video_mode));
                window_attributes.resizable = false;
            }
        } else {
            log::info!("No primary monitor found; using windowed mode.");
            window_attributes.inner_size =
                Some(Size::new(winit::dpi::LogicalSize::new(1920.0, 1080.0)));
        }

        window_attributes.visible = false;

        Arc::new(event_loop.create_window(window_attributes).unwrap())
    }

    async fn create_wgpu(
        window: Arc<winit::window::Window>,
    ) -> anyhow::Result<(
        wgpu::Surface<'static>,
        wgpu::Device,
        wgpu::Queue,
        wgpu::SurfaceConfiguration,
    )> {
        log::info!("Initializing WGPU");

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .ok_or(anyhow::anyhow!("Failed to find suitable surface format"))?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Ok((surface, device, queue, config))
    }

    pub fn create_shader_module(&self, label: &str, source: &str) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    pub fn create_buffer(
        &self,
        label: &str,
        data: &[u8],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage,
            })
    }

    pub fn create_render_pipeline(
        &self,
        vertex: VertexShader,
        fragment: FragmentShader,
    ) -> wgpu::RenderPipeline {
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: vertex.module,
                    entry_point: vertex.entry_point,
                    buffers: vertex.buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: fragment.module,
                    entry_point: fragment.entry_point,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
    }
}
