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

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, 1.0, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

impl gravsim::application::Application for GravSimApp {
    fn new(ws: &mut gravsim::window_surface::WindowSurface<Self>) -> Self {
        let shader = ws.create_shader_module("Shader", include_str!("shader.wgsl"));
        let render_pipeline = ws.create_render_pipeline(
            VertexShader {
                module: &shader,
                buffers: &[Vertex::desc()],
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

    fn render(&mut self, context: &mut gravsim::window_surface::RenderContext) {
        context.render_pass(
            gravsim::window_surface::RenderPassDesc {
                label: Some("Main Render Pass"),
                clear_color: wgpu::Color::BLACK,
            },
            |pass| {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_vertex_buffer(0, self.wgpu_buffer.slice(..));
                pass.draw(0..VERTICES.len() as u32, 0..1);
            },
        );
    }

    fn ui(&mut self, ui: &mut imgui::Ui) {
        let mut showed = true;
        ui.show_demo_window(&mut showed);
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
