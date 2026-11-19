//! Player cube + world-origin gizmo (the only two object shaders in use).

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::world::{BlockSideTexture, Side};

/// `min_uniform_buffer_offset_alignment` is at most 256 in WebGPU.
const SLOT: u64 = 256;
const CUBE_SLOT: u32 = 0;
const GIZMO_SLOT: u32 = 1;

pub struct ObjectPipelines {
    cube: wgpu::RenderPipeline,
    gizmo: wgpu::RenderPipeline,
    cube_vertices: wgpu::Buffer,
    cube_count: u32,
    gizmo_vertices: wgpu::Buffer,
    gizmo_count: u32,
    /// Model matrices: slot 0 = cube, slot 1 = gizmo (dynamic offset).
    model_buf: wgpu::Buffer,
    model_bind: wgpu::BindGroup,
}

impl ObjectPipelines {
    pub fn new(
        device: &wgpu::Device,
        globals_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("object model"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("objects"),
            bind_group_layouts: &[Some(globals_layout), Some(&model_layout)],
            immediate_size: 0,
        });
        let make = |label: &'static str,
                    src: &str,
                    topology,
                    cull_mode,
                    attributes: &[wgpu::VertexAttribute]| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes,
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let cube = make(
            "cube",
            include_str!("../../../shaders/cube.wgsl"),
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32],
        );
        // GL parity: gizmo has depth test but no culling and no blending.
        let gizmo = make(
            "gizmo",
            include_str!("../../../shaders/gizmo.wgsl"),
            wgpu::PrimitiveTopology::LineList,
            None,
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        );

        let cube_data = cube_vertices();
        let cube_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube vertices"),
            contents: bytemuck::cast_slice(&cube_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let gizmo_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gizmo vertices"),
            contents: bytemuck::cast_slice(&GIZMO_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let model_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("object models"),
            size: 2 * SLOT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let model_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("object model"),
            layout: &model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &model_buf,
                    offset: 0,
                    size: wgpu::BufferSize::new(64),
                }),
            }],
        });
        Self {
            cube,
            gizmo,
            cube_vertices,
            cube_count: (cube_data.len() / 6) as u32,
            gizmo_vertices,
            gizmo_count: (GIZMO_VERTICES.len() / 6) as u32,
            model_buf,
            model_bind,
        }
    }

    /// Upload this frame's model matrices (a missing object keeps its old slot, undrawn).
    pub fn write_models(&self, queue: &wgpu::Queue, cube: Option<&Mat4>, gizmo: Option<&Mat4>) {
        for (slot, m) in [(CUBE_SLOT, cube), (GIZMO_SLOT, gizmo)] {
            if let Some(m) = m {
                queue.write_buffer(
                    &self.model_buf,
                    slot as u64 * SLOT,
                    bytemuck::cast_slice(&m.to_cols_array()),
                );
            }
        }
    }

    /// Group 0 (globals) must already be bound.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, cube: bool, gizmo: bool) {
        if cube {
            pass.set_pipeline(&self.cube);
            pass.set_bind_group(1, &self.model_bind, &[CUBE_SLOT * SLOT as u32]);
            pass.set_vertex_buffer(0, self.cube_vertices.slice(..));
            pass.draw(0..self.cube_count, 0..1);
        }
        if gizmo {
            pass.set_pipeline(&self.gizmo);
            pass.set_bind_group(1, &self.model_bind, &[GIZMO_SLOT * SLOT as u32]);
            pass.set_vertex_buffer(0, self.gizmo_vertices.slice(..));
            pass.draw(0..self.gizmo_count, 0..1);
        }
    }
}

/// x y z r g b per vertex, `LineList`.
const GIZMO_VERTICES: [f32; 36] = [
    0.0, 0.0, 0.0, 1.0, 0.0, 0.0, //
    1.0, 0.0, 0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 0.0, 1.0, //
    0.0, 0.0, 1.0, 0.0, 0.0, 1.0, //
];

const fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}
//   C  D     G  H
// A  B     E  F
const A: Vec3 = v(-1.0, 1.0, 1.0);
const B: Vec3 = v(1.0, 1.0, 1.0);
const C: Vec3 = v(-1.0, 1.0, -1.0);
const D: Vec3 = v(1.0, 1.0, -1.0);
const E: Vec3 = v(-1.0, -1.0, 1.0);
const F: Vec3 = v(1.0, -1.0, 1.0);
const G: Vec3 = v(-1.0, -1.0, -1.0);
const H: Vec3 = v(1.0, -1.0, -1.0);

/// Corners `[a, b, c, d]` per side, indexed by `Side as usize` (same as the GL cube).
const SIDE_CORNERS: [[Vec3; 4]; 6] = [
    [C, A, B, D],
    [E, G, H, F],
    [A, E, F, B],
    [D, H, G, C],
    [B, F, H, D],
    [C, G, E, A],
];

/// x y z u v layer per vertex, `TriangleList`: two triangles per side.
fn cube_vertices() -> Vec<f32> {
    let sides = [
        (Side::Top, BlockSideTexture::GrassTop),
        (Side::Bottom, BlockSideTexture::Dirt),
        (Side::Front, BlockSideTexture::GrassSide),
        (Side::Back, BlockSideTexture::GrassSide),
        (Side::Left, BlockSideTexture::GrassSide),
        (Side::Right, BlockSideTexture::GrassSide),
    ];
    // uv: top-left (0,0), bottom-left (0,1), bottom-right (1,1), top-right (1,0)
    let (tl, bl, br, tr) = ((0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0));
    let mut out = Vec::with_capacity(6 * 6 * 6);
    for (side, texture) in sides {
        let [a, b, c, d] = SIDE_CORNERS[side as usize];
        for (p, (u, w)) in [(a, tl), (b, bl), (c, br), (a, tl), (c, br), (d, tr)] {
            out.extend_from_slice(&[p.x, p.y, p.z, u, w, texture as usize as f32]);
        }
    }
    out
}
