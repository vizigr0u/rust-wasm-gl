//! Player cube + world-origin gizmo (the only two object shaders in use).

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::world::{BlockSideTexture, Side};

/// `min_uniform_buffer_offset_alignment` is at most 256 in WebGPU.
const SLOT: u64 = 256;
const CUBE_SLOT: u32 = 0;
const GIZMO_SLOT: u32 = 1;
const FRUSTUM_SLOT: u32 = 2;
/// Culled-chunk boxes use slots `BOX_SLOT0..`.
const BOX_SLOT0: u32 = 3;

pub struct ObjectPipelines {
    cube: wgpu::RenderPipeline,
    gizmo: wgpu::RenderPipeline,
    /// Gizmo shader, no depth test: the frustum lines show through terrain.
    lines: wgpu::RenderPipeline,
    frustum_vertices: wgpu::Buffer,
    frustum_count: u32,
    box_vertices: wgpu::Buffer,
    box_count: u32,
    model_layout: wgpu::BindGroupLayout,
    model_slots: u64,
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
                    depth: bool,
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
                    depth_write_enabled: Some(depth),
                    depth_compare: Some(if depth {
                        wgpu::CompareFunction::Less
                    } else {
                        wgpu::CompareFunction::Always
                    }),
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
            true,
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32],
        );
        // GL parity: gizmo has depth test but no culling and no blending.
        let gizmo = make(
            "gizmo",
            include_str!("../../../shaders/gizmo.wgsl"),
            wgpu::PrimitiveTopology::LineList,
            None,
            true,
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        );
        let lines = make(
            "debug lines",
            include_str!("../../../shaders/gizmo.wgsl"),
            wgpu::PrimitiveTopology::LineList,
            None,
            false,
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        );
        let frustum_data = frustum_vertices();
        let frustum_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frustum vertices"),
            contents: bytemuck::cast_slice(&frustum_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let box_data = box_vertices();
        let box_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("box vertices"),
            contents: bytemuck::cast_slice(&box_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

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
        let model_slots = 64;
        let (model_buf, model_bind) = create_model_buf(device, &model_layout, model_slots);
        Self {
            cube,
            gizmo,
            lines,
            frustum_vertices,
            frustum_count: (frustum_data.len() / 6) as u32,
            box_vertices,
            box_count: (box_data.len() / 6) as u32,
            model_layout,
            model_slots,
            cube_vertices,
            cube_count: (cube_data.len() / 6) as u32,
            gizmo_vertices,
            gizmo_count: (GIZMO_VERTICES.len() / 6) as u32,
            model_buf,
            model_bind,
        }
    }

    /// Upload this frame's model matrices (a missing object keeps its old slot, undrawn).
    pub fn write_models(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cube: Option<&Mat4>,
        gizmo: Option<&Mat4>,
        frustum: Option<&Mat4>,
        boxes: &[Mat4],
    ) {
        let needed = BOX_SLOT0 as u64 + boxes.len() as u64;
        if needed > self.model_slots {
            self.model_slots = needed.next_power_of_two();
            (self.model_buf, self.model_bind) =
                create_model_buf(device, &self.model_layout, self.model_slots);
        }
        for (slot, m) in [
            (CUBE_SLOT, cube),
            (GIZMO_SLOT, gizmo),
            (FRUSTUM_SLOT, frustum),
        ] {
            if let Some(m) = m {
                queue.write_buffer(
                    &self.model_buf,
                    slot as u64 * SLOT,
                    bytemuck::cast_slice(&m.to_cols_array()),
                );
            }
        }
        if !boxes.is_empty() {
            let mut bytes = vec![0u8; boxes.len() * SLOT as usize];
            for (i, m) in boxes.iter().enumerate() {
                let at = i * SLOT as usize;
                bytes[at..at + 64].copy_from_slice(bytemuck::cast_slice(&m.to_cols_array()));
            }
            queue.write_buffer(&self.model_buf, BOX_SLOT0 as u64 * SLOT, &bytes);
        }
    }

    /// Group 0 (globals) must already be bound.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        cube: bool,
        gizmo: bool,
        frustum: bool,
        boxes: u32,
    ) {
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
        if boxes > 0 {
            // Same pipeline as the gizmo: depth tested.
            pass.set_pipeline(&self.gizmo);
            pass.set_vertex_buffer(0, self.box_vertices.slice(..));
            for i in 0..boxes {
                pass.set_bind_group(1, &self.model_bind, &[(BOX_SLOT0 + i) * SLOT as u32]);
                pass.draw(0..self.box_count, 0..1);
            }
        }
        if frustum {
            pass.set_pipeline(&self.lines);
            pass.set_bind_group(1, &self.model_bind, &[FRUSTUM_SLOT * SLOT as u32]);
            pass.set_vertex_buffer(0, self.frustum_vertices.slice(..));
            pass.draw(0..self.frustum_count, 0..1);
        }
    }
}

fn create_model_buf(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    slots: u64,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("object models"),
        size: slots * SLOT,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("object model"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buf,
                offset: 0,
                size: wgpu::BufferSize::new(64),
            }),
        }],
    });
    (buf, bind)
}

/// 12 edges of a cube with corners at +-1 (x, y) and `z0`..`z1`, in one colour.
fn cube_edges(z0: f32, z1: f32, rgb: [f32; 3], out: &mut Vec<f32>) {
    let corner = |i: usize, z: f32| {
        [
            if i & 1 == 0 { -1.0 } else { 1.0 },
            if i & 2 == 0 { -1.0 } else { 1.0 },
            z,
        ]
    };
    let mut push = |p: [f32; 3]| {
        out.extend_from_slice(&p);
        out.extend_from_slice(&rgb);
    };
    // near ring (0,1,3,2), far ring, and 4 connectors
    for (i, j) in [(0, 1), (1, 3), (3, 2), (2, 0)] {
        push(corner(i, z0));
        push(corner(j, z0));
        push(corner(i, z1));
        push(corner(j, z1));
    }
    for i in 0..4 {
        push(corner(i, z0));
        push(corner(i, z1));
    }
}

/// NDC volume of WebGPU clip space (z 0..1): yellow edges plus a cyan forward ray.
fn frustum_vertices() -> Vec<f32> {
    let mut out = Vec::new();
    cube_edges(0.0, 1.0, [1.0, 1.0, 0.0], &mut out);
    out.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
    out.extend_from_slice(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0]);
    out
}

/// Red cube edges at +-1, scaled and moved per culled chunk.
fn box_vertices() -> Vec<f32> {
    let mut out = Vec::new();
    cube_edges(-1.0, 1.0, [1.0, 0.0, 0.0], &mut out);
    out
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
