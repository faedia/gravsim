use std::sync::Arc;

use wgpu::{RenderPassColorAttachment, RenderPassDescriptor};
use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{self, Window},
};

struct WindowSurface {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    window: Arc<Window>,
    last_frame_time: std::time::Instant,
}

impl WindowSurface {
    async fn new(window: Arc<Window>) -> WindowSurface {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Or ```rs
        // let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        // ```
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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
        });

        WindowSurface {
            surface: surface,
            device: device,
            queue: queue,
            config: config,
            render_pipeline: render_pipeline,
            window: window,
            last_frame_time: std::time::Instant::now(),
        }
    }

    fn render(&mut self) {
        self.window.request_redraw();

        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.last_frame_time = std::time::Instant::now();
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == self.config.width && height == self.config.height {
            return;
        }
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}

#[derive(Default)]
struct App {
    window_surface: Option<WindowSurface>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let start_time = std::time::Instant::now();
        log::info!("Application resumed; creating window.");
        let mut window_attributes = Window::default_attributes();
        window_attributes.title = "GravSim Example".to_string();

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

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window_surface = Some(pollster::block_on(WindowSurface::new(window.clone())));

        self.window_surface
            .as_mut()
            .unwrap()
            .resize(window.inner_size().width, window.inner_size().height);

        window.set_visible(true);
        window.focus_window();
        window
            .set_cursor_grab(window::CursorGrabMode::Confined)
            .ok();

        log::info!("Window created.");
        log::info!(
            "Time taken to create window and surface: {:?}",
            start_time.elapsed()
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::trace!("Closing window {:?}", window_id);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                log::trace!("Redrawing window {:?}", window_id);
                if self
                    .window_surface
                    .as_ref()
                    .unwrap()
                    .window
                    .is_minimized()
                    .unwrap()
                {
                    log::trace!("Window {:?} is minimized; skipping redraw.", window_id);
                    return;
                }
                self.window_surface.as_mut().unwrap().render();
            }
            WindowEvent::Resized(size) => {
                log::trace!("Resizing window {:?} to {:?}", window_id, size);
                self.window_surface
                    .as_mut()
                    .unwrap()
                    .resize(size.width, size.height);
            }
            WindowEvent::Focused(focused) => {
                log::trace!("Window {:?} focused: {}", window_id, focused);
                if let Some(ws) = self.window_surface.as_mut()
                    && ws.window.fullscreen().is_some()
                {
                    self.window_surface
                        .as_mut()
                        .unwrap()
                        .window
                        .set_minimized(!focused);
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
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
                    let ws = self.window_surface.as_mut().unwrap();
                    if ws.window.fullscreen().is_some() {
                        ws.window.set_fullscreen(None);
                        let _ =
                            ws.window
                                .request_inner_size(Size::new(winit::dpi::LogicalSize::new(
                                    1920.0, 1080.0,
                                )));
                        ws.resize(1920, 1080);
                    } else {
                        if let Some(monitor) = event_loop.primary_monitor() {
                            let first_mode = monitor.video_modes().next();
                            if let Some(video_mode) = first_mode {
                                let _ = ws.window.request_inner_size(Size::new(
                                    winit::dpi::PhysicalSize {
                                        width: video_mode.size().width,
                                        height: video_mode.size().height,
                                    },
                                ));

                                ws.resize(video_mode.size().width, video_mode.size().height);

                                ws.window.set_fullscreen(Some(window::Fullscreen::Exclusive(
                                    video_mode,
                                )));
                            }
                        }
                    }
                }
            }
            _ => log::trace!("Skipping event {:?}", event),
        }
    }
}

fn main() {
    env_logger::init();
    log::info!("Starting application.");

    let event_loop = EventLoop::with_user_event().build().unwrap();

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();

    log::info!("Application closing.");
}
