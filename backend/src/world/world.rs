use std::{collections::HashMap, mem::size_of, rc::Rc};

use glam::{ivec3, IVec3, Vec3};
use glow::HasContext;
use log::info;

const MAX_LOAD_CHUNK_DISTANCE: i32 = 15;
const MAX_LOAD_CHUNK_DISTANCE_SQUARED: i32 = MAX_LOAD_CHUNK_DISTANCE * MAX_LOAD_CHUNK_DISTANCE;
const MAX_MESH_TO_KEEP: usize = 1024;

// const CHUNK_LOADING_WEIGHT: f32 = 1.0;
// const CHUNK_GENERATION_WEIGHT: f32 = 0.5;
const MAX_LOADS_PER_FRAME: usize = 32;

use crate::{core::Time, graphics::Camera, graphics::TextureType, world::WorldGenerator};

use super::{BlockPos, ChunkPos, ChunkStreamer, ChunkVao, WorldRenderData};

type OffsetPriority = Vec<IVec3>;

fn make_offset_priority(size: i32) -> OffsetPriority {
    let total_offsets = size * size * size;
    let mut offsets = Vec::with_capacity(total_offsets as _);
    for z in -size + 1..size {
        for y in -size + 1..size {
            for x in -size + 1..size {
                offsets.push(ivec3(x, y, z));
            }
        }
    }
    offsets.sort_by(|p, q| p.length_squared().cmp(&q.length_squared()));
    info!(
        "offset: {}, {}, ..., {}, {}",
        offsets[0],
        offsets[1],
        offsets[total_offsets as usize - 2],
        offsets[total_offsets as usize - 1]
    );
    offsets
}

#[derive(Debug)]
pub struct World<G>
where
    G: WorldGenerator,
{
    chunks: HashMap<ChunkPos, Option<ChunkVao>>,
    loaded_vertices: usize,
    // loaded_meshes: Vec<LoadedChunkMesh>,
    streamer: ChunkStreamer<G>,
    last_computed_chunk_pos: Option<ChunkPos>,
    offset_priority: OffsetPriority,
    chunks_to_load: Vec<ChunkPos>,
    render_data: WorldRenderData,
}

impl<G> World<G>
where
    G: WorldGenerator,
{
    pub fn new(generator: G) -> Self {
        info!("World: Creating");
        let offset_priority = make_offset_priority(MAX_LOAD_CHUNK_DISTANCE);

        Self {
            chunks: HashMap::new(),
            loaded_vertices: 0,
            streamer: ChunkStreamer::new(generator),
            offset_priority,
            last_computed_chunk_pos: None,
            chunks_to_load: Vec::with_capacity(MAX_MESH_TO_KEEP * 2),
            render_data: WorldRenderData::new(),
        }
    }

    pub fn get_info(&self) -> String {
        let vertex_count = self.loaded_vertices;
        let memory_used = (vertex_count * size_of::<f32>()) as f32 / 1000000.0;
        let loaded_meshes = self.chunks.values().map(|c| c.is_some()).count();
        format!(
            "World: Loaded {}/{} chunks\nStreaming: {}\n{vertex_count} vertices - {memory_used:.3} MB",
            loaded_meshes,
            self.chunks.len(),
            self.streamer.get_info(),
        )
    }

    pub fn setup_graphics(
        &mut self,
        gl: &glow::Context,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) -> Result<(), String> {
        self.render_data.setup_graphics(gl, texture)
    }

    pub fn update(
        &mut self,
        gl: &glow::Context,
        _time: &Time,
        player_pos: Vec3,
    ) -> Result<(), String> {
        let player_block_pos: BlockPos = player_pos.as_ivec3().into();
        let player_chunk_pos: ChunkPos = player_block_pos.into();

        let mut geom_changed = false;
        if self.streamer.tick_streaming(player_chunk_pos) > 0 {
            geom_changed = true;
        }

        if geom_changed || self.last_computed_chunk_pos != Some(player_chunk_pos) {
            self.last_computed_chunk_pos = Some(player_chunk_pos);
            self.on_chunk_changed(player_chunk_pos, gl);
            geom_changed = true;
        }

        if self.load_some_chunks(gl) > 0 {
            geom_changed = true;
        }

        if geom_changed {
            self.render_data.compile(gl, player_chunk_pos, &self.chunks);
        }

        Ok(())
    }

    fn load_some_chunks(&mut self, gl: &glow::Context) -> usize {
        if self.chunks_to_load.len() == 0 {
            return 0;
        }
        let chunks_to_load: Vec<ChunkPos> = self
            .chunks_to_load
            .iter()
            .take(MAX_LOADS_PER_FRAME)
            .copied()
            .collect();
        let chunks_count_to_load = chunks_to_load.len();
        info!(
            "World Graphics: gl loading {} chunks this frame",
            chunks_count_to_load
        );

        if chunks_count_to_load > 0 {
            for chunk_pos in chunks_to_load {
                if !self.chunks.contains_key(&chunk_pos) {
                    let chunk = match self.streamer.get_chunk(chunk_pos) {
                        None => None,
                        Some(chunk) => {
                            let mesh = ChunkVao::load(gl, chunk).expect("can't load mesh");
                            self.loaded_vertices += mesh.vertex_count;
                            Some(mesh)
                        }
                    };
                    self.chunks.insert(chunk_pos, chunk);
                }
            }
            self.chunks_to_load.retain(|c| !self.chunks.contains_key(c));
        }

        chunks_count_to_load
    }

    pub fn on_chunk_changed(&mut self, new_chunk_pos: ChunkPos, gl: &glow::Context) {
        info!("World: recompute for chunk pos: {new_chunk_pos:?}");

        // delete some chunks
        {
            let mut chunks_we_can_unload = Vec::with_capacity(MAX_MESH_TO_KEEP * 2);
            for (&chunk_pos, c) in self.chunks.iter() {
                if let Some(_) = &c {
                    if new_chunk_pos.distance_squared(chunk_pos) > MAX_LOAD_CHUNK_DISTANCE_SQUARED {
                        chunks_we_can_unload.push(chunk_pos);
                    }
                }
            }
            chunks_we_can_unload.sort_by(|a, b| {
                a.distance_squared(new_chunk_pos)
                    .cmp(&b.distance_squared(new_chunk_pos))
            });
            if chunks_we_can_unload.len() > MAX_MESH_TO_KEEP {
                info!(
                    "World: unloading {} chunks",
                    chunks_we_can_unload.len() - MAX_MESH_TO_KEEP
                );
                for chunk_pos in chunks_we_can_unload.drain(MAX_MESH_TO_KEEP..) {
                    let vao = self
                        .chunks
                        .remove(&chunk_pos)
                        .expect("added inexistent chunk")
                        .expect("added empty chunk");
                    unsafe {
                        gl.delete_vertex_array(vao.vertex_array);
                    }
                    self.loaded_vertices -= vao.vertex_count;
                }
            }
        }

        // add new chunks
        {
            for offset in &self.offset_priority {
                let chunk_pos: ChunkPos = (new_chunk_pos.as_vec() + *offset).into();
                if !self.chunks.contains_key(&chunk_pos)
                    // && self.streamer.is_chunked_streamed(chunk_pos)
                    && self.streamer.get_chunk(chunk_pos).is_some()
                {
                    self.chunks_to_load.push(chunk_pos);
                }
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        self.render_data.render(gl, camera);
    }
}
