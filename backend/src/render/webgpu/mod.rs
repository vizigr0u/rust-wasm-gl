//! wgpu renderer (WebGPU only). Port in progress: C6 = egui done.

mod gpuprobe;
mod objects;

use std::collections::HashMap;

use web_sys::{HtmlCanvasElement, HtmlImageElement};

use super::{FrameInput, RenderError, RenderStats, TextureId, UiFrame};
use crate::world::{ChunkDrawData, ChunkPos};
use voxel_core::gpu::{
    quad_indices, DrawEntry, DrawList, RangeAllocator, SlotAllocator, MAX_QUADS_PER_DRAW,
};
use voxel_core::render::{cull_draws, pack_sides, DrawListCache, SideRanges};

/// Initial arena size in quads (4 B each = 8 MB).
const ARENA_QUADS: u32 = 2 << 20;
const ARENA_USAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE
    .union(wgpu::BufferUsages::COPY_DST)
    .union(wgpu::BufferUsages::COPY_SRC);

fn create_arena(device: &wgpu::Device, quads: u32) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quad arena"),
        size: quads as u64 * 4,
        usage: ARENA_USAGE,
        mapped_at_creation: false,
    })
}

/// Bytes per chunk table entry: `{pos: vec3<i32>}` padded to 16.
const CHUNK_ENTRY: u64 = 16;
/// view + projection (128 B) + player chunk (`vec3<i32>`, padded to 16 B).
const GLOBALS_SIZE: u64 = 144;
/// Draw slots per chunk, one per side: `slot = chunk_slot * 6 + side`, passed as `first_instance`.
const SIDES_PER_CHUNK: u32 = 6;

/// One chunk's quads in the arena: all 6 sides contiguous.
struct GpuChunk {
    /// Static per-chunk slot: its entry in the chunk table holds the position, written at upload.
    slot: u32,
    range: std::ops::Range<u32>,
    /// Absolute quad indices into the arena.
    side_ranges: SideRanges,
}

struct ChunkPipeline {
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
    chunk_layout: wgpu::BindGroupLayout,
    globals_buf: wgpu::Buffer,
    /// Static `4k+{0,1,2,2,1,3}` indices for `MAX_QUADS_PER_DRAW` quads.
    index_buf: wgpu::Buffer,
    /// Per-chunk `{pos}` entries indexed by chunk slot. Written at upload, grown on demand.
    chunk_buf: wgpu::Buffer,
    /// Capacity in chunk entries.
    chunk_cap: u64,
    /// Arena + chunk table. Recreated whenever either buffer is.
    chunk_bind: wgpu::BindGroup,
    /// Needs the Blocks texture, so created once it is loaded.
    globals_bind: Option<wgpu::BindGroup>,
}

fn create_chunk_buf(device: &wgpu::Device, entries: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chunk table"),
        size: entries * CHUNK_ENTRY,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_chunk_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    arena: &wgpu::Buffer,
    chunk_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("chunk data"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: arena.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: chunk_buf.as_entire_binding(),
            },
        ],
    })
}

impl ChunkPipeline {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        arena: &wgpu::Buffer,
        format: wgpu::TextureFormat,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/chunk.wgsl").into()),
        });
        let buffer = |binding, ty, min| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(min),
            },
            count: None,
        };
        let storage = wgpu::BufferBindingType::Storage { read_only: true };
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chunk globals"),
            entries: &[
                buffer(0, wgpu::BufferBindingType::Uniform, GLOBALS_SIZE),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let chunk_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chunk data"),
            entries: &[buffer(0, storage, 4), buffer(1, storage, CHUNK_ENTRY)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk"),
            bind_group_layouts: &[Some(&globals_layout), Some(&chunk_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chunk globals"),
            size: GLOBALS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let indices = quad_indices(MAX_QUADS_PER_DRAW);
        let index_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad indices"),
            size: indices.len() as u64 * 2,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buf, 0, bytemuck::cast_slice(&indices));
        let chunk_cap = 64;
        let chunk_buf = create_chunk_buf(device, chunk_cap);
        let chunk_bind = create_chunk_bind(device, &chunk_layout, arena, &chunk_buf);
        Self {
            pipeline,
            globals_layout,
            chunk_layout,
            globals_buf,
            index_buf,
            chunk_buf,
            chunk_cap,
            chunk_bind,
            globals_bind: None,
        }
    }
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    egui_renderer: egui_wgpu::Renderer,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    /// Loaded textures (non-sRGB, 1 mip).
    textures: HashMap<TextureId, (wgpu::Texture, wgpu::TextureView)>,
    /// Nearest/Nearest, Repeat: GL parity.
    sampler: wgpu::Sampler,
    chunk_pipeline: ChunkPipeline,
    objects: objects::ObjectPipelines,
    gpu_chunks: HashMap<ChunkPos, GpuChunk>,
    chunk_slots: SlotAllocator,
    arena: wgpu::Buffer,
    alloc: RangeAllocator,
    draw_list: DrawList,
    draw_cache: DrawListCache,
    /// Bytes written to the arena since the last `render`.
    uploaded_bytes: u64,
    draw_calls: u32,
    /// Chunk-box model matrices for this frame's culled chunks (empty unless shown).
    culled_boxes: Vec<glam::Mat4>,
    cull_counts: (u32, u32, u32),
    times: RenderStats,
    probe: gpuprobe::GpuProbe,
    /// Adapter description, for profile reports.
    adapter_info: String,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("config", &self.config)
            .finish()
    }
}

fn create_depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

impl Renderer {
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, RenderError> {
        let (w, h) = (canvas.width().max(1), canvas.height().max(1));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| RenderError(format!("create_surface: {e}")))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError(format!("WebGPU not available: {e}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rwg device"),
                // Optional: only for profiling GPU time. Absent = no GPU timings.
                required_features: adapter.features() & gpuprobe::GpuProbe::FEATURE,
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError(format!("request_device: {e}")))?;

        let probe = gpuprobe::GpuProbe::new(&device, &queue);
        let adapter_info = format!("{:?}", adapter.get_info());
        device.set_device_lost_callback(|reason, msg| {
            log::error!("wgpu device lost ({reason:?}): {msg}");
        });
        device.on_uncaptured_error(std::sync::Arc::new(|err: wgpu::Error| {
            log::error!("wgpu error: {err}");
        }));

        // Non-sRGB so colours match the WebGL2 default framebuffer (no gamma).
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .ok_or("no non-sRGB surface format")?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let depth_view = create_depth_view(&device, w, h);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest repeat"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let arena = create_arena(&device, ARENA_QUADS);
        let chunk_pipeline = ChunkPipeline::new(&device, &queue, &arena, format);
        let objects = objects::ObjectPipelines::new(
            &device,
            &chunk_pipeline.globals_layout,
            format,
            DEPTH_FORMAT,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            format,
            egui_wgpu::RendererOptions {
                dithering: false,
                ..Default::default()
            },
        );
        Ok(Self {
            egui_renderer,
            chunk_pipeline,
            objects,
            gpu_chunks: HashMap::new(),
            chunk_slots: SlotAllocator::new(),
            arena,
            alloc: RangeAllocator::new(ARENA_QUADS),
            draw_list: DrawList::default(),
            draw_cache: DrawListCache::default(),
            uploaded_bytes: 0,
            draw_calls: 0,
            culled_boxes: Vec::new(),
            cull_counts: (0, 0, 0),
            times: RenderStats::default(),
            probe,
            adapter_info,
            textures: HashMap::new(),
            sampler,
            surface,
            device,
            queue,
            config,
            depth_view,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width, height) == (self.config.width, self.config.height) {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_view(&self.device, width, height);
    }

    /// Upload a decoded image. `layers` > 1: the image is a vertical strip of `layers` tiles.
    pub fn load_texture(&mut self, id: TextureId, img: &HtmlImageElement, layers: u32) {
        let layers = layers.max(1);
        let (w, h) = (img.natural_width(), img.natural_height() / layers);
        if w == 0 || h == 0 {
            log::error!("texture {id:?}: empty image");
            return;
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{id:?}")),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        for layer in 0..layers {
            self.queue.copy_external_image_to_texture(
                &wgpu::CopyExternalImageSourceInfo {
                    source: wgpu::ExternalImageSource::HTMLImageElement(img.clone()),
                    origin: wgpu::Origin2d { x: 0, y: layer * h },
                    flip_y: false,
                },
                wgpu::CopyExternalImageDestInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                    color_space: wgpu::PredefinedColorSpace::Srgb,
                    premultiplied_alpha: false,
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(if layers > 1 {
                wgpu::TextureViewDimension::D2Array
            } else {
                wgpu::TextureViewDimension::D2
            }),
            ..Default::default()
        });
        self.textures.insert(id, (texture, view));
        if id == TextureId::Blocks {
            self.chunk_pipeline.globals_bind = None;
        }
    }

    /// Double the arena (at least) until `needed` more quads fit, keeping every range.
    fn grow_arena(&mut self, needed: u32) {
        let old = self.alloc.capacity();
        let mut new = old * 2;
        while new - old < needed {
            new *= 2;
        }
        let buf = create_arena(&self.device, new);
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.copy_buffer_to_buffer(&self.arena, 0, &buf, 0, old as u64 * 4);
        self.queue.submit([enc.finish()]);
        self.arena = buf;
        let cp = &mut self.chunk_pipeline;
        cp.chunk_bind =
            create_chunk_bind(&self.device, &cp.chunk_layout, &self.arena, &cp.chunk_buf);
        self.alloc.grow(new);
        log::info!("quad arena grown: {old} -> {new} quads");
    }

    /// Write one chunk's quads into the arena, replacing its previous range.
    pub fn upload_chunk(&mut self, pos: ChunkPos, data: &ChunkDrawData) {
        self.remove_chunk(pos);
        if data.quad_count() == 0 {
            return;
        }
        let (all, mut side_ranges) = pack_sides(data);
        let n = all.len() as u32;
        let range = match self.alloc.alloc(n) {
            Some(r) => r,
            None => {
                self.grow_arena(RangeAllocator::rounded(n));
                self.alloc.alloc(n).expect("arena grown")
            }
        };
        for r in &mut side_ranges {
            r.0 += range.start;
        }
        self.queue.write_buffer(
            &self.arena,
            range.start as u64 * 4,
            bytemuck::cast_slice(&all),
        );
        self.uploaded_bytes += all.len() as u64 * 4;
        let slot = self.chunk_slots.alloc();
        self.ensure_chunk_slots();
        self.write_chunk_entry(slot, pos);
        self.draw_cache.invalidate();
        self.gpu_chunks.insert(
            pos,
            GpuChunk {
                slot,
                range,
                side_ranges,
            },
        );
    }

    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        if let Some(c) = self.gpu_chunks.remove(&pos) {
            self.draw_cache.invalidate();
            self.alloc.free(c.range);
            self.chunk_slots.free(c.slot);
        }
    }

    fn write_chunk_entry(&self, slot: u32, pos: ChunkPos) {
        let v = pos.as_vec();
        self.queue.write_buffer(
            &self.chunk_pipeline.chunk_buf,
            slot as u64 * CHUNK_ENTRY,
            bytemuck::cast_slice(&[v.x, v.y, v.z, 0]),
        );
    }

    /// Grow the chunk table to cover every allocated chunk slot; rewrites all live chunks.
    fn ensure_chunk_slots(&mut self) {
        let needed = self.chunk_slots.capacity() as u64;
        let cp = &mut self.chunk_pipeline;
        if needed <= cp.chunk_cap {
            return;
        }
        cp.chunk_cap = needed.next_power_of_two();
        cp.chunk_buf = create_chunk_buf(&self.device, cp.chunk_cap);
        cp.chunk_bind =
            create_chunk_bind(&self.device, &cp.chunk_layout, &self.arena, &cp.chunk_buf);
        for (pos, c) in &self.gpu_chunks {
            self.write_chunk_entry(c.slot, *pos);
        }
    }

    /// Upload globals and rebuild `draw_list`.
    fn prepare_chunks(&mut self, frame: &FrameInput) {
        let cp = &mut self.chunk_pipeline;
        if cp.globals_bind.is_none() {
            let Some((_, view)) = self.textures.get(&TextureId::Blocks) else {
                self.draw_list.clear();
                return;
            };
            cp.globals_bind = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("chunk globals"),
                layout: &cp.globals_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: cp.globals_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));
        }
        let mut globals = [0u8; GLOBALS_SIZE as usize];
        let mats: Vec<f32> = [frame.view, frame.projection]
            .iter()
            .flat_map(|m| m.to_cols_array())
            .collect();
        globals[..128].copy_from_slice(bytemuck::cast_slice(&mats));
        let pc = frame.player_chunk.as_vec();
        globals[128..140].copy_from_slice(bytemuck::cast_slice(&[pc.x, pc.y, pc.z]));
        self.queue.write_buffer(&cp.globals_buf, 0, &globals);

        // Floating origin: each chunk's offset from the player's chunk (exact in f32).
        let draws = self.draw_cache.get(
            self.gpu_chunks.iter().map(|(p, c)| (*p, c.side_ranges)),
            frame.player_chunk,
        );
        let (draws, culled) = cull_draws(draws, frame.cull_frustum.as_ref());
        let half = voxel_core::world::CHUNK_SIZE as f32 / 2.0;
        self.culled_boxes.clear();
        if frame.culled_boxes {
            self.culled_boxes.extend(culled.iter().map(|o| {
                glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::splat(half),
                    glam::Quat::IDENTITY,
                    *o + glam::Vec3::splat(half),
                )
            }));
        }
        let mut visible_chunks = 0;
        let mut prev = None;
        for d in &draws {
            if prev != Some(d.chunk) {
                prev = Some(d.chunk);
                visible_chunks += 1;
            }
        }
        self.cull_counts = (
            visible_chunks,
            culled.len() as u32,
            draws.iter().map(|d| d.count).sum(),
        );
        self.draw_list.clear();
        let mut last_chunk = None;
        let mut chunk_slot = 0u32;
        for d in draws {
            if last_chunk != Some(d.chunk) {
                last_chunk = Some(d.chunk);
                chunk_slot = self.gpu_chunks[&d.chunk].slot;
            }
            self.draw_list.push(
                DrawEntry {
                    slot: chunk_slot * SIDES_PER_CHUNK + d.side as u32,
                    first: d.first,
                    count: d.count,
                },
                true,
            );
        }
    }

    pub fn render(&mut self, frame: &FrameInput, ui: Option<UiFrame>) {
        let t0 = crate::utils::performance_now();
        use wgpu::CurrentSurfaceTexture as Cst;
        let tex = match self.surface.get_current_texture() {
            Cst::Success(t) => t,
            Cst::Suboptimal(t) => {
                self.surface.configure(&self.device, &self.config);
                t
            }
            Cst::Outdated | Cst::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Cst::Timeout | Cst::Occluded => return,
            other => {
                log::error!("surface texture: {other:?}");
                return;
            }
        };
        let view = tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let t1 = crate::utils::performance_now();
        self.prepare_chunks(frame);
        self.draw_calls = self.draw_list.entries.len() as u32;
        self.objects.write_models(
            &self.device,
            &self.queue,
            frame.player_model.as_ref(),
            frame.gizmo_model.as_ref(),
            frame.frustum_lines.as_ref(),
            &self.culled_boxes,
        );
        let t2 = crate::utils::performance_now();
        let [r, g, b, a] = frame.clear_color.map(f64::from);
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.probe.pass_writes(),
                ..Default::default()
            });
            if let Some(globals) = &self.chunk_pipeline.globals_bind {
                let cp = &self.chunk_pipeline;
                pass.set_bind_group(0, globals, &[]);
                if !self.draw_list.entries.is_empty() {
                    pass.set_pipeline(&cp.pipeline);
                    pass.set_bind_group(1, &cp.chunk_bind, &[]);
                    pass.set_index_buffer(cp.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                    // Draw id = first_instance (one instance); quads are pulled from the arena
                    // at `base_vertex / 4 + index / 4`.
                    for e in &self.draw_list.entries {
                        pass.draw_indexed(0..6 * e.count, 4 * e.first as i32, e.slot..e.slot + 1);
                    }
                }
                self.objects.draw(
                    &mut pass,
                    frame.player_model.is_some(),
                    frame.gizmo_model.is_some(),
                    frame.frustum_lines.is_some(),
                    self.culled_boxes.len() as u32,
                );
            }
        }
        self.probe.resolve(&mut enc);
        let t3 = crate::utils::performance_now();
        let mut extra = Vec::new();
        if let Some(ui) = &ui {
            for (id, deltas) in &ui.textures_delta.set {
                for delta in deltas {
                    self.egui_renderer
                        .update_texture(&self.device, &self.queue, *id, delta);
                }
            }
            let screen = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: 1.0,
            };
            extra = self.egui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut enc,
                &ui.primitives,
                &screen,
            );
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.egui_renderer
                .render(&mut pass, &ui.primitives, &screen);
        }
        let t4 = crate::utils::performance_now();
        self.queue.submit(extra.into_iter().chain([enc.finish()]));
        self.queue.present(tex);
        self.probe.after_submit(&self.queue, t4);
        if let Some(mut ui) = ui {
            for id in &ui.textures_delta.free {
                self.egui_renderer.free_texture(id);
            }
            // epaint's Drop debug-asserts the delta is empty; mark it handled.
            ui.textures_delta.clear();
        }
        let t5 = crate::utils::performance_now();
        self.times = RenderStats {
            draw_calls: 0,
            chunk_buffers: 0,
            draw_entries_raw: self.draw_list.pushed,
            chunks_visible: self.cull_counts.0,
            chunks_culled: self.cull_counts.1,
            quads_drawn: self.cull_counts.2,
            arena_used: self.alloc.used(),
            arena_capacity: self.alloc.capacity(),
            arena_largest_free: self.alloc.largest_free(),
            upload_bytes: std::mem::take(&mut self.uploaded_bytes),
            acquire_ms: t1 - t0,
            prepare_ms: t2 - t1,
            pass_ms: t3 - t2,
            egui_ms: t4 - t3,
            submit_ms: t5 - t4,
            marks: [t0, t1, t2, t3, t4, t5],
        };
    }

    /// GPU readbacks of the last submitted frame still in flight.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn gpu_busy(&self) -> bool {
        self.probe.busy()
    }

    /// `(main pass ms, submit -> done ms)` of the last finished frame, NaN when not available.
    /// Each reading is returned once.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn collect_gpu(&mut self) -> (f64, f64) {
        (self.probe.take_pass_ms(), self.probe.take_done_ms())
    }

    /// Adapter description and whether GPU timestamp queries are available.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn gpu_report(&self) -> (String, bool) {
        (self.adapter_info.clone(), self.probe.has_timestamps())
    }

    pub fn stats(&self) -> RenderStats {
        RenderStats {
            draw_calls: self.draw_calls,
            chunk_buffers: self.gpu_chunks.len(),
            ..self.times
        }
    }

    /// Compile every shader and build every pipeline inside validation error scopes.
    /// One message per failure; empty = all good.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn compile_check(&self) -> impl std::future::Future<Output = Vec<String>> + 'static {
        let device = self.device.clone();
        let queue = self.queue.clone();
        let format = self.config.format;
        async move {
            let mut errors = Vec::new();
            for (name, src) in [
                ("chunk", include_str!("../../../shaders/chunk.wgsl")),
                ("cube", include_str!("../../../shaders/cube.wgsl")),
                ("gizmo", include_str!("../../../shaders/gizmo.wgsl")),
            ] {
                let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(name),
                    source: wgpu::ShaderSource::Wgsl(src.into()),
                });
                for m in module.get_compilation_info().await.messages {
                    if m.message_type == wgpu::CompilationMessageType::Error {
                        errors.push(format!("{name}: {}", m.message));
                    }
                }
                if let Some(e) = scope.pop().await {
                    errors.push(format!("{name}: {e}"));
                }
            }
            let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
            let arena = create_arena(&device, 1024);
            let chunk = ChunkPipeline::new(&device, &queue, &arena, format);
            let _objects =
                objects::ObjectPipelines::new(&device, &chunk.globals_layout, format, DEPTH_FORMAT);
            if let Some(e) = scope.pop().await {
                errors.push(format!("pipelines: {e}"));
            }
            errors
        }
    }
}
