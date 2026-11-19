use std::{collections::HashMap, mem::size_of};

use glam::{ivec3, IVec3};
use log::info;

use crate::world::{Neighbours, Side, WorldGenerator, MAX_CHUNK_Y, MIN_CHUNK_Y};

use super::{ChunkDrawData, ChunkPos, ChunkStreamer};

/// Streaming loads whole 6x20x6 pages in a 3x3 ring around the player's page, so the
/// guaranteed horizontal reach is one page = 6 chunks. Load radius matches that.
const LOAD_RADIUS: i32 = 6;
/// Unload radius > load radius (hysteresis): a chunk loaded at the edge is not
/// unloaded by the next small move, so crossing chunk borders does not churn meshes.
const UNLOAD_RADIUS: i32 = LOAD_RADIUS + 2;
const LOAD_RADIUS_SQUARED: i32 = LOAD_RADIUS * LOAD_RADIUS;
const UNLOAD_RADIUS_SQUARED: i32 = UNLOAD_RADIUS * UNLOAD_RADIUS;

const MAX_LOADS_PER_FRAME: usize = 8;

/// Chunk neighbour directions, indexed by `Side as usize`.
const SIDE_DIRS: [IVec3; 6] = [
    ivec3(0, 1, 0),  // Top
    ivec3(0, -1, 0), // Bottom
    ivec3(0, 0, 1),  // Front
    ivec3(0, 0, -1), // Back
    ivec3(1, 0, 0),  // Right
    ivec3(-1, 0, 0), // Left
];

type OffsetPriority = Vec<IVec3>;

/// All offsets within `radius` (a sphere: the same metric as unloading), nearest first.
fn make_offset_priority(radius: i32) -> OffsetPriority {
    let side = 2 * radius + 1;
    let mut offsets = Vec::with_capacity((side * side * side) as usize);
    for z in -radius..=radius {
        for y in -radius..=radius {
            for x in -radius..=radius {
                let o = ivec3(x, y, z);
                if o.length_squared() <= radius * radius {
                    offsets.push(o);
                }
            }
        }
    }
    offsets.sort_by_key(|p| p.length_squared());
    offsets
}

/// A mesh plus whether all 6 neighbours were available when it was built. An
/// incomplete mesh treated missing neighbours as air and is remeshed once they stream in.
#[derive(Debug)]
struct MeshEntry {
    draw_data: ChunkDrawData,
    complete: bool,
}

/// Mesh changes since the last `update`. The renderer applies them to the GPU.
#[derive(Debug, Default)]
pub struct MeshChanges {
    /// Meshes created or rebuilt: fetch with `World::mesh`.
    pub added: Vec<ChunkPos>,
    pub removed: Vec<ChunkPos>,
}

#[derive(Debug)]
pub struct World<G>
where
    G: WorldGenerator,
{
    meshes: HashMap<ChunkPos, MeshEntry>,
    loaded_quads: usize,
    streamer: ChunkStreamer<G>,
    last_computed_chunk_pos: Option<ChunkPos>,
    offset_priority: OffsetPriority,
    chunks_to_load: Vec<ChunkPos>,
    player_chunk_pos: ChunkPos,
}

/// Each quad is one packed `i32` word in `ChunkDrawData::vertex_data`.
const QUAD_DATA_SIZE: usize = size_of::<i32>();

fn in_world_height(pos: IVec3) -> bool {
    pos.y >= MIN_CHUNK_Y && pos.y < MAX_CHUNK_Y
}

impl<G> World<G>
where
    G: WorldGenerator,
{
    pub fn new(generator: G) -> Self {
        info!("World: Creating");
        Self {
            meshes: HashMap::new(),
            loaded_quads: 0,
            streamer: ChunkStreamer::new(generator),
            offset_priority: make_offset_priority(LOAD_RADIUS),
            last_computed_chunk_pos: None,
            chunks_to_load: Vec::new(),
            player_chunk_pos: ChunkPos::default(),
        }
    }

    pub fn get_info(&self, gpu_info: &str) -> String {
        let quad_count = self.loaded_quads;
        let memory_used = (quad_count * QUAD_DATA_SIZE) as f32 / 1000000.0;
        format!(
            "{} chunk meshes\nStreaming: {}\n{quad_count} quads - {memory_used:.3} MB\n{gpu_info}",
            self.meshes.len(),
            self.streamer.get_info(),
        )
    }

    /// Streams and meshes. Returns the mesh changes for the caller to mirror on the GPU.
    pub fn update(&mut self, player_chunk_pos: ChunkPos) -> MeshChanges {
        self.player_chunk_pos = player_chunk_pos;
        let mut changes = MeshChanges::default();

        let streaming = self.streamer.tick_streaming(player_chunk_pos);
        // Single owner: the streamer owns block data, so meshes of freed pages go too.
        for page in &streaming.pages_freed {
            let freed: Vec<ChunkPos> = self
                .meshes
                .keys()
                .filter(|c| page.has_chunk_pos(**c))
                .copied()
                .collect();
            for pos in freed {
                self.remove_mesh(pos, &mut changes);
            }
        }

        if streaming.pages_loaded > 0 || self.last_computed_chunk_pos != Some(player_chunk_pos) {
            self.last_computed_chunk_pos = Some(player_chunk_pos);
            self.on_chunk_changed(player_chunk_pos, &mut changes);
        }

        self.load_some_chunks(&mut changes);
        changes
    }

    pub fn mesh(&self, pos: ChunkPos) -> Option<&ChunkDrawData> {
        self.meshes.get(&pos).map(|m| &m.draw_data)
    }

    pub fn player_chunk_pos(&self) -> ChunkPos {
        self.player_chunk_pos
    }

    fn remove_mesh(&mut self, pos: ChunkPos, changes: &mut MeshChanges) {
        if let Some(entry) = self.meshes.remove(&pos) {
            self.loaded_quads -= entry.draw_data.quad_count();
            changes.removed.push(pos);
        }
    }

    /// Neighbours of `pos` plus whether all are known. Outside the world height is
    /// known air; a not-yet-streamed neighbour is unknown.
    fn gather_neighbours(&self, pos: ChunkPos) -> (Neighbours<'_>, bool) {
        let mut result: Neighbours = [None; 6];
        let mut complete = true;
        for side in [
            Side::Top,
            Side::Bottom,
            Side::Front,
            Side::Back,
            Side::Right,
            Side::Left,
        ] {
            let n = pos.as_vec() + SIDE_DIRS[side as usize];
            if !in_world_height(n) {
                continue;
            }
            match self.streamer.get_chunk(n.into()) {
                Some(c) => result[side as usize] = Some(c),
                None => complete = false,
            }
        }
        (result, complete)
    }

    fn needs_mesh(&self, pos: ChunkPos) -> bool {
        if !self.streamer.is_chunked_streamed(pos) {
            return false;
        }
        match self.meshes.get(&pos) {
            None => true,
            Some(entry) => !entry.complete && self.gather_neighbours(pos).1,
        }
    }

    fn load_some_chunks(&mut self, changes: &mut MeshChanges) {
        if self.chunks_to_load.is_empty() {
            return;
        }
        let todo: Vec<ChunkPos> = self
            .chunks_to_load
            .iter()
            .filter(|c| self.needs_mesh(**c))
            .take(MAX_LOADS_PER_FRAME)
            .copied()
            .collect();

        for pos in todo {
            let Some(chunk) = self.streamer.get_chunk(pos) else {
                continue;
            };
            let (neighbours, complete) = self.gather_neighbours(pos);
            let draw_data = ChunkDrawData::load(chunk, &neighbours);
            self.loaded_quads += draw_data.quad_count();
            if let Some(old) = self.meshes.insert(
                pos,
                MeshEntry {
                    draw_data,
                    complete,
                },
            ) {
                self.loaded_quads -= old.draw_data.quad_count();
            }
            changes.added.push(pos);
        }
        // Keep only what still needs work (new, or incomplete and now completable).
        let still: Vec<ChunkPos> = self
            .chunks_to_load
            .iter()
            .filter(|c| self.needs_mesh(**c))
            .copied()
            .collect();
        self.chunks_to_load = still;
    }

    fn on_chunk_changed(&mut self, new_chunk_pos: ChunkPos, changes: &mut MeshChanges) {
        info!("World: recompute for chunk pos: {new_chunk_pos:?}");

        // Unload with the same metric as load plus hysteresis.
        let too_far: Vec<ChunkPos> = self
            .meshes
            .keys()
            .filter(|c| new_chunk_pos.distance_squared(**c) > UNLOAD_RADIUS_SQUARED)
            .copied()
            .collect();
        if !too_far.is_empty() {
            info!("World: unloading {} chunk meshes", too_far.len());
        }
        for pos in too_far {
            self.remove_mesh(pos, changes);
        }

        // Rebuild the queue (nearest first). Everything queued is within LOAD_RADIUS,
        // so nothing just unloaded (beyond UNLOAD_RADIUS) can be re-queued.
        self.chunks_to_load.clear();
        for offset in &self.offset_priority {
            let v = new_chunk_pos.as_vec() + *offset;
            if !in_world_height(v) {
                continue;
            }
            let pos: ChunkPos = v.into();
            if self.needs_mesh(pos) {
                self.chunks_to_load.push(pos);
            }
        }
    }
}

const _: () = assert!(LOAD_RADIUS_SQUARED < UNLOAD_RADIUS_SQUARED);
