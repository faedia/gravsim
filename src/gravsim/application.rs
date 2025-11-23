use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop};

use crate::gravsim::window_surface::{RenderContext, WindowSurface};

/// The Application trait defines the interface for applications
/// that can be run using the gravsim framework.
///
/// Each application that implements that trait can be run using the `run_app` function.
/// ```rust
/// struct MyApp {}
///
/// impl gravsim::application::Application for MyApp {
///     fn new(ws: &mut gravsim::window_surface::WindowSurface<Self>) -> Self { MyApp {} }
///     fn render(&mut self, context: gravsim::window_surface::RenderContext<Self>) {}
/// }
///
/// gravsim::application::run_app::<MyApp>().unwrap()
/// ```
pub trait Application: Sized {
    /// Creates a new instance of the application.
    /// The `WindowSurface` is provided to allow the application access to windowing and rendering functionality.
    /// This function is called once the application has started and the window and rendering context are ready.
    fn new(ws: &mut WindowSurface<Self>) -> Self;

    /// Renders a frame for the application.
    /// This function is called every frame to allow the application to render its content.
    fn render(&mut self, context: &mut RenderContext);

    fn ui(&mut self, ui: &mut imgui::Ui);
}

struct ApplicationWrapper<App: Application> {
    window_surface: Option<WindowSurface<App>>,
}

impl<App: Application> ApplicationHandler for ApplicationWrapper<App> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_surface.is_some() {
            return;
        }
        let ws = WindowSurface::new(event_loop);
        self.window_surface = Some(pollster::block_on(ws));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(ws) = &mut self.window_surface {
            match ws.handle_event(event_loop, window_id, event) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error handling window event: {:?}", e);
                }
            }
        }
    }
}

/// Runs the application of the specified type `App` that implements the `Application` trait.
/// This function initializes the event loop and window surface,
/// and starts the application by calling its `new` method.
/// The application will then handle rendering and events through the event loop.
///
/// ```rust
/// struct MyApp {}
///
/// impl gravsim::application::Application for MyApp {
///     fn new(ws: &mut gravsim::window_surface::WindowSurface<Self>) -> Self { MyApp {} }
///     fn render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {}
/// }
///
/// gravsim::application::run_app::<MyApp>().unwrap()
/// ```
pub fn run_app<App: Application>() -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app_wrapper = ApplicationWrapper::<App> {
        window_surface: None,
    };
    event_loop.run_app(&mut app_wrapper)?;

    Ok(())
}
