use std::{collections::HashMap, mem::size_of, rc::Rc};

use glam::{ivec3, IVec3, Vec3};
use log::info;

const FRAME_RENDER_DELAY: usize = 8;

const MAX_LOAD_CHUNK_DISTANCE: i32 = 15;
const MAX_LOAD_CHUNK_DISTANCE_SQUARED: i32 = MAX_LOAD_CHUNK_DISTANCE * MAX_LOAD_CHUNK_DISTANCE;
const MAX_MESH_TO_KEEP: usize = 1024;

// const CHUNK_LOADING_WEIGHT: f32 = 1.0;
// const CHUNK_GENERATION_WEIGHT: f32 = 0.5;
const MAX_LOADS_PER_FRAME: usize = 8;

use crate::{
    core::Time,
    graphics::{Camera, TextureType},
    world::{WorldGenerator, MAX_CHUNK_Y, MIN_CHUNK_Y},
};

use super::{ChunkDrawData, ChunkPos, ChunkStreamer, WorldRenderData};

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
    // info!(
    //     "offset: {}, {}, ..., {}, {}",
    //     offsets[0],
    //     offsets[1],
    //     offsets[total_offsets as usize - 2],
    //     offsets[total_offsets as usize - 1]
    // );
    offsets
}

#[derive(Debug)]
pub struct World<G>
where
    G: WorldGenerator,
{
    chunks: HashMap<ChunkPos, Option<ChunkDrawData>>,
    loaded_quads: usize,
    // loaded_meshes: Vec<LoadedChunkMesh>,
    streamer: ChunkStreamer<G>,
    last_computed_chunk_pos: Option<ChunkPos>,
    offset_priority: OffsetPriority,
    chunks_to_load: Vec<ChunkPos>,
    render_data: WorldRenderData,
    render_frame_delay: usize,
    render_dirty: bool,
    player_chunk_pos: ChunkPos,
}

const QUAD_DATA_SIZE: usize = size_of::<f32>() + size_of::<Vec3>();

impl<G> World<G>
where
    G: WorldGenerator,
{
    pub fn new(generator: G) -> Self {
        info!("World: Creating");
        let offset_priority = make_offset_priority(MAX_LOAD_CHUNK_DISTANCE);

        Self {
            chunks: HashMap::new(),
            loaded_quads: 0,
            streamer: ChunkStreamer::new(generator),
            offset_priority,
            last_computed_chunk_pos: None,
            chunks_to_load: Vec::with_capacity(MAX_MESH_TO_KEEP * 2),
            render_data: WorldRenderData::new(),
            render_frame_delay: 0,
            render_dirty: false,
            player_chunk_pos: ChunkPos::default(),
        }
    }

    pub fn get_info(&self) -> String {
        let quad_count = self.loaded_quads;
        let memory_used = (quad_count * QUAD_DATA_SIZE) as f32 / 1000000.0;
        let loaded_meshes = self.chunks.values().filter(|c| c.is_some()).count();
        let (peak_v, peak_l) = self.render_data.peak_cache_len();
        format!(
            "Loaded {loaded_meshes}/{} chunks\nStreaming: {}\n{quad_count} vertices - {memory_used:.3} MB\nPeak gather: {peak_v} verts, {peak_l} locs",
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
        player_chunk_pos: ChunkPos,
    ) -> Result<(), String> {
        self.player_chunk_pos = player_chunk_pos;
        // let player_block_pos: BlockPos = player_pos.as_ivec3().into();
        // let player_chunk_pos: ChunkPos = player_block_pos.into();

        let mut geom_changed = false;
        if self.streamer.tick_streaming(player_chunk_pos) > 0 {
            geom_changed = true;
        }

        if geom_changed || self.last_computed_chunk_pos != Some(player_chunk_pos) {
            self.last_computed_chunk_pos = Some(player_chunk_pos);
            self.on_chunk_changed(player_chunk_pos, gl);
            geom_changed = true;
        }

        if self.load_some_chunks() > 0 {
            geom_changed = true;
        }

        if geom_changed {
            self.render_dirty = true;
        }
        if self.render_frame_delay > 0 {
            self.render_frame_delay -= 1;
        }
        if self.render_dirty && self.render_frame_delay == 0 {
            self.render_dirty = false;
            self.render_data
                .gather_draw_data(gl, player_chunk_pos, &self.chunks);
            self.render_frame_delay = FRAME_RENDER_DELAY;
        }

        Ok(())
    }

    fn load_some_chunks(&mut self) -> usize {
        if self.chunks_to_load.len() == 0 {
            return 0;
        }
        let chunks_to_load: Vec<ChunkPos> = self
            .chunks_to_load
            .iter()
            .filter(|c| !self.chunks.contains_key(c) && self.streamer.is_chunked_streamed(**c))
            .take(MAX_LOADS_PER_FRAME)
            .copied()
            .collect();
        let mut loaded_count = 0;

        for chunk_pos in chunks_to_load {
            // Page may have been freed: leave it queued, never insert None.
            if let Some(chunk) = self.streamer.get_chunk(chunk_pos) {
                let draw_data = ChunkDrawData::load(chunk).expect("can't load mesh");
                self.loaded_quads += draw_data.quad_count();
                self.chunks.insert(chunk_pos, Some(draw_data));
                loaded_count += 1;
            }
        }
        self.chunks_to_load.retain(|c| !self.chunks.contains_key(c));

        loaded_count
    }

    pub fn on_chunk_changed(&mut self, new_chunk_pos: ChunkPos, _gl: &glow::Context) {
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
                    let draw_data = self
                        .chunks
                        .remove(&chunk_pos)
                        .expect("added inexistent chunk")
                        .expect("added empty chunk");
                    self.loaded_quads -= draw_data.quad_count();
                }
            }
        }

        // add new chunks
        {
            // Rebuild queue: avoids duplicates and drops stale far chunks.
            self.chunks_to_load.clear();
            for offset in &self.offset_priority {
                let chunk_vec_pos = new_chunk_pos.as_vec() + *offset;
                if chunk_vec_pos.y < MIN_CHUNK_Y || chunk_vec_pos.y >= MAX_CHUNK_Y {
                    continue;
                }
                let chunk_pos: ChunkPos = chunk_vec_pos.into();
                if !self.chunks.contains_key(&chunk_pos)
                    && self.streamer.is_chunked_streamed(chunk_pos)
                // && self.streamer.get_chunk(chunk_pos).is_some()
                {
                    self.chunks_to_load.push(chunk_pos);
                }
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        self.render_data.render(gl, camera, self.player_chunk_pos);
    }
}
