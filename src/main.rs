use std::sync::Arc;

use wgpu::{RenderPassColorAttachment, RenderPassDescriptor};
use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::{self, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{self, Window},
};

struct WindowSurface {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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

        WindowSurface {
            surface: surface,
            device: device,
            queue: queue,
            config: config,
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

        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.last_frame_time = std::time::Instant::now();
    }

    fn resize(&mut self, width: u32, height: u32) {
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
                window_attributes.decorations = false;
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
                self.window_surface
                    .as_mut()
                    .unwrap()
                    .window
                    .set_minimized(!focused);
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
