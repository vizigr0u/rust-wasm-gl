use crate::gamestate::GameState;
use egui::mutex::Mutex;
use log::info;
use std::{borrow::Cow, mem, sync::Arc};
use web_sys::OffscreenCanvas;
use wgpu::{util::DeviceExt, SurfaceTarget};

const DEFAULT_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.4,
    g: 0.6,
    b: 0.7,
    a: 1.0,
};

struct DrawState {
    clear_color: wgpu::Color,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
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

pub struct GraphicsContext<'w> {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'w>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    draw_state: DrawState,
    render_pipeline: wgpu::RenderPipeline,
    game_state: Arc<Mutex<GameState>>,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    gui_renderer: egui_wgpu::Renderer,
}

impl<'w> GraphicsContext<'w> {
    pub async fn new(canvas: OffscreenCanvas, game_state: Arc<Mutex<GameState>>) -> Self {
        let instance = wgpu::Instance::default();
        let (width, height) = (canvas.width(), canvas.height());

        let surface = instance
            .create_surface(SurfaceTarget::OffscreenCanvas(canvas))
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let config = surface.get_default_config(&adapter, width, height).unwrap();
        surface.configure(&device, &config);

        // Load the shaders from disk
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/shader.wgsl"))),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let gui_renderer = egui_wgpu::Renderer::new(&device, swapchain_format, None, 1);

        Self {
            instance,
            adapter,
            surface,
            device,
            queue,
            config,
            draw_state: DrawState {
                clear_color: DEFAULT_CLEAR_COLOR,
            },
            render_pipeline,
            game_state,
            gui_renderer,
            vertex_buffer,
            num_vertices: VERTICES.len() as u32,
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.draw_state.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.draw(0..self.num_vertices, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));

        // self.gui_renderer.render(render_pass, paint_jobs, screen_descriptor)

        frame.present();

        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.update();
        self.render()
    }

    fn update(&mut self) {
        let state = self.game_state.lock();
        self.draw_state.clear_color = wgpu::Color {
            r: state.mouse_pos.0 / self.config.width as f64, //self.size.width as f64,
            g: state.mouse_pos.1 / self.config.height as f64, // self.size.height as f64,
            b: 1.0,
            a: 1.0,
        };
    }

    pub fn reset_swap_chain(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        info!("resizing to {:?}", new_size);
        // Reconfigure the surface with the new size
        self.config.width = new_size.width.max(1);
        self.config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.config);
    }
}
