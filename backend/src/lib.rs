use log::info;
use std::borrow::Cow;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const DEFAULT_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.4,
    g: 0.6,
    b: 0.7,
    a: 1.0,
};

struct GraphicsContext<'w> {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'w>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    clear_color: wgpu::Color,
    window: &'w Window,
    render_pipeline: wgpu::RenderPipeline,
}

impl<'w> GraphicsContext<'w> {
    async fn new(window: &'w Window) -> Self {
        let mut size = window.inner_size();
        info!("window size: {size:?}");
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::default();

        let surface = instance
            .create_surface(window)
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
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        // Load the shaders from disk
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/shader.wgsl"))),
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
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            instance,
            adapter,
            surface,
            device,
            queue,
            config,
            clear_color: DEFAULT_CLEAR_COLOR,
            window,
            render_pipeline,
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
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.clear_color = wgpu::Color {
                    r: position.x as f64 / self.config.width as f64, //self.size.width as f64,
                    g: position.y as f64 / self.config.height as f64, // self.size.height as f64,
                    b: 1.0,
                    a: 1.0,
                };
                true
            }
            _ => false,
        }
    }

    fn update(&mut self) {}

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        info!("resizing to {:?}", new_size);
        // Reconfigure the surface with the new size
        self.config.width = new_size.width.max(1);
        self.config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.config);
    }
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let window = &window;
    let mut state = GraphicsContext::new(&window).await;
    event_loop
        .run(|event, event_loop_window_target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match &event {
                        WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                        WindowEvent::ScaleFactorChanged { .. } => {
                            state.resize(window.inner_size());
                        }
                        WindowEvent::CloseRequested => event_loop_window_target.exit(),
                        WindowEvent::KeyboardInput {
                            event: key_event, ..
                        } => {
                            if state.input(event) {
                                window.request_redraw();
                            }
                            if key_event.state.is_pressed() {
                                match key_event.physical_key {
                                    PhysicalKey::Code(KeyCode::Escape) => {
                                        event_loop_window_target.exit();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            state.update();
                            match state.render() {
                                Ok(_) => {}
                                // Recreate the swap_chain if lost
                                Err(wgpu::SurfaceError::Lost) => state.resize(PhysicalSize::new(
                                    state.config.width,
                                    state.config.height,
                                )),
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    event_loop_window_target.exit()
                                }
                                Err(e) => eprintln!("Some unhandled error {:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                state.window.request_redraw();
            }
            _ => {}
        })
        .unwrap()
}

use wasm_bindgen::prelude::*;
use winit::platform::web::WindowBuilderExtWebSys;

#[wasm_bindgen(start)]
pub fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(800, 600))
        .with_title("WebGPU-based DVR")
        .with_append(true)
        .build(&event_loop)
        .unwrap();
    // let _ = window.request_inner_size(PhysicalSize::new(800, 600));

    // #[cfg(not(target_arch = "wasm32"))]
    // {
    //     env_logger::init();
    //     pollster::block_on(run(event_loop, window));
    // }
    // #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
