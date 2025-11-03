use crate::gravsim::shader::{FragmentShader, VertexShader};

mod gravsim;

struct GravSimApp {
    render_pipeline: wgpu::RenderPipeline,
    wgpu_buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

impl gravsim::application::Application for GravSimApp {
    fn new(ws: &mut gravsim::window_surface::WindowSurface<Self>) -> Self {
        let shader = ws.create_shader_module("Shader", include_str!("shader.wgsl"));
        let render_pipeline = ws.create_render_pipeline(
            VertexShader {
                module: &shader,
                entry_point: Some("vs_main"),
            },
            FragmentShader {
                module: &shader,
                entry_point: Some("fs_main"),
            },
        );

        let wgpu_buffer = ws.create_buffer(
            "Vertex Buffer",
            bytemuck::cast_slice(VERTICES),
            wgpu::BufferUsages::VERTEX,
        );

        GravSimApp {
            render_pipeline: render_pipeline,
            wgpu_buffer,
        }
    }

    fn render(&mut self, context: &mut gravsim::window_surface::RenderContext<Self>) {
        context.render_pass(
            gravsim::window_surface::RenderPassDesc {
                label: Some("Main Render Pass"),
                clear_color: wgpu::Color::BLACK,
            },
            |pass| {
                pass.set_pipeline(&self.render_pipeline);
                pass.draw(0..3, 0..1);
            },
        );
    }
}

fn main() {
    env_logger::init();
    log::info!("Starting application.");

    let exit_sate = gravsim::application::run_app::<GravSimApp>();

    if let Err(e) = exit_sate {
        log::error!("Application exited with error: {:?}", e);
        std::process::exit(1);
    }

    log::info!("Application closing.");
}
