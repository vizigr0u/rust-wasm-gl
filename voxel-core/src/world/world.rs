use std::{collections::HashMap, mem::size_of};

use glam::{ivec3, IVec3};
use log::info;

use super::{
    decode_mesh_output, encode_mesh_input, ChunkDrawData, ChunkPos, ChunkStreamer, InlineMesher,
    MeshExecutor, MeshJob, Neighbours, Side, WorldGenerator, MAX_CHUNK_Y, MIN_CHUNK_Y,
};

/// Streaming loads whole 6x20x6 pages in a 3x3 ring around the player's page, so the
/// guaranteed horizontal reach is one page = 6 chunks. Load radius matches that.
const LOAD_RADIUS: i32 = 6;
/// Unload radius > load radius (hysteresis): a chunk loaded at the edge is not
/// unloaded by the next small move, so crossing chunk borders does not churn meshes.
const UNLOAD_RADIUS: i32 = LOAD_RADIUS + 2;
const LOAD_RADIUS_SQUARED: i32 = LOAD_RADIUS * LOAD_RADIUS;
const UNLOAD_RADIUS_SQUARED: i32 = UNLOAD_RADIUS * UNLOAD_RADIUS;

const MAX_LOADS_PER_FRAME: usize = 8;
/// Mesh jobs submitted but not finished. Bounds the work queued behind a slow worker.
const MAX_IN_FLIGHT: usize = 16;

/// Chunk neighbour directions, indexed by `Side as usize`.
const SIDE_DIRS: [IVec3; 6] = [
    ivec3(0, 1, 0),  // Top
    ivec3(0, -1, 0), // Bottom
    ivec3(0, 0, 1),  // Front
    ivec3(0, 0, -1), // Back
    ivec3(1, 0, 0),  // Right
    ivec3(-1, 0, 0), // Left
];

pub type OffsetPriority = Vec<IVec3>;

/// All offsets within `radius` (a sphere: the same metric as unloading), nearest first.
pub fn make_offset_priority(radius: i32) -> OffsetPriority {
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

/// A submitted mesh job. `complete`: all 6 neighbours were known when it was encoded.
#[derive(Debug)]
struct InFlight {
    id: u64,
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
    mesher: Box<dyn MeshExecutor>,
    /// Chunks being meshed. A result whose id no longer matches is stale and dropped.
    in_flight: HashMap<ChunkPos, InFlight>,
    next_job_id: u64,
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
        Self::with_mesher(generator, Box::<InlineMesher>::default())
    }

    pub fn with_mesher(generator: G, mesher: Box<dyn MeshExecutor>) -> Self {
        info!("World: Creating");
        Self {
            meshes: HashMap::new(),
            mesher,
            in_flight: HashMap::new(),
            next_job_id: 0,
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
            "{} chunk meshes ({} meshing)\nStreaming: {}\n{quad_count} quads - {memory_used:.3} MB\n{gpu_info}",
            self.meshes.len(),
            self.in_flight.len(),
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
                .chain(self.in_flight.keys())
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

        self.load_some_chunks();
        self.collect_meshes(&mut changes);
        changes
    }

    /// Nothing left to stream or mesh for the current player position.
    pub fn is_settled(&self) -> bool {
        self.streamer.is_idle() && self.chunks_to_load.is_empty() && self.in_flight.is_empty()
    }

    pub fn mesh(&self, pos: ChunkPos) -> Option<&ChunkDrawData> {
        self.meshes.get(&pos).map(|m| &m.draw_data)
    }

    pub fn player_chunk_pos(&self) -> ChunkPos {
        self.player_chunk_pos
    }

    fn remove_mesh(&mut self, pos: ChunkPos, changes: &mut MeshChanges) {
        // A job for this chunk would resurrect it (or mesh freed block data).
        self.in_flight.remove(&pos);
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
        if self.in_flight.contains_key(&pos) {
            return false;
        }
        match self.meshes.get(&pos) {
            None => true,
            Some(entry) => !entry.complete && self.gather_neighbours(pos).1,
        }
    }

    /// Encode and submit mesh jobs for the nearest chunks that need one.
    fn load_some_chunks(&mut self) {
        if self.chunks_to_load.is_empty() {
            return;
        }
        let budget = MAX_LOADS_PER_FRAME.min(MAX_IN_FLIGHT.saturating_sub(self.in_flight.len()));
        let todo: Vec<ChunkPos> = self
            .chunks_to_load
            .iter()
            .filter(|c| self.needs_mesh(**c))
            .take(budget)
            .copied()
            .collect();

        for pos in todo {
            let Some(chunk) = self.streamer.get_chunk(pos) else {
                continue;
            };
            let (neighbours, complete) = self.gather_neighbours(pos);
            let input = encode_mesh_input(chunk, &neighbours);
            let id = self.next_job_id;
            self.next_job_id += 1;
            self.in_flight.insert(pos, InFlight { id, complete });
            self.mesher.submit(MeshJob { id, pos, input });
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

    /// Store finished meshes. Results of unloaded or superseded jobs are dropped.
    fn collect_meshes(&mut self, changes: &mut MeshChanges) {
        for result in self.mesher.poll() {
            let Some(pos) = self
                .in_flight
                .iter()
                .find(|(_, f)| f.id == result.id)
                .map(|(p, _)| *p)
            else {
                continue;
            };
            let flight = self.in_flight.remove(&pos).expect("found above");
            let draw_data = decode_mesh_output(&result.output);
            self.loaded_quads += draw_data.quad_count();
            if let Some(old) = self.meshes.insert(
                pos,
                MeshEntry {
                    draw_data,
                    complete: flight.complete,
                },
            ) {
                self.loaded_quads -= old.draw_data.quad_count();
            }
            changes.added.push(pos);
        }
    }

    fn on_chunk_changed(&mut self, new_chunk_pos: ChunkPos, changes: &mut MeshChanges) {
        info!("World: recompute for chunk pos: {new_chunk_pos:?}");

        // Unload with the same metric as load plus hysteresis.
        let too_far: Vec<ChunkPos> = self
            .meshes
            .keys()
            .chain(self.in_flight.keys())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::hash_words;
    use crate::world::GeneratorV0;

    #[test]
    fn offset_priority_snapshot() {
        let p = make_offset_priority(LOAD_RADIUS);
        assert!(p
            .windows(2)
            .all(|w| w[0].length_squared() <= w[1].length_squared()));
        assert_eq!(p[0], ivec3(0, 0, 0));
        assert!(p.iter().all(|o| o.length_squared() <= LOAD_RADIUS_SQUARED));
        let words: Vec<i32> = p.iter().flat_map(|o| [o.x, o.y, o.z]).collect();
        let small: Vec<String> = make_offset_priority(1)
            .iter()
            .map(|o| o.to_string())
            .collect();
        insta::assert_yaml_snapshot!((p.len(), hash_words(&words), small));
    }

    fn settle(w: &mut World<GeneratorV0>, p: ChunkPos) -> (usize, usize) {
        let (mut added, mut removed) = (0, 0);
        for _ in 0..2000 {
            let c = w.update(p);
            added += c.added.len();
            removed += c.removed.len();
        }
        (added, removed)
    }

    #[test]
    fn world_settles_and_hysteresis_prevents_churn() {
        let mut w = World::new(GeneratorV0::new());
        let (a0, r0) = settle(&mut w, ivec3(0, 0, 0).into());
        let info0 = w.get_info("gpu");
        let (quads0, meshes0) = (w.loaded_quads, w.meshes.len());
        // settled: further ticks do nothing
        assert_eq!(settle(&mut w, ivec3(0, 0, 0).into()), (0, 0));

        let step = settle(&mut w, ivec3(1, 0, 0).into());
        // stepping back must not unload anything (hysteresis)
        let back = settle(&mut w, ivec3(0, 0, 0).into());
        assert_eq!(back.1, 0, "no churn when stepping back");

        let mut walk = (0, 0);
        for x in 1..=20 {
            let r = settle(&mut w, ivec3(x, 0, 0).into());
            walk = (walk.0 + r.0, walk.1 + r.1);
        }
        insta::assert_yaml_snapshot!((
            (a0, r0, meshes0, quads0, info0),
            (step, back),
            (walk, w.meshes.len(), w.loaded_quads),
        ));
        let real: usize = w.meshes.values().map(|m| m.draw_data.quad_count()).sum();
        assert_eq!(real, w.loaded_quads);
    }

    /// Holds finished meshes back until `release`, like a busy worker.
    #[derive(Debug, Default)]
    struct DelayedMesher {
        inner: InlineMesher,
        held: Vec<super::super::MeshResult>,
        release: std::rc::Rc<std::cell::Cell<bool>>,
        max_pending: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl MeshExecutor for DelayedMesher {
        fn submit(&mut self, job: MeshJob) {
            self.inner.submit(job);
            self.held.extend(self.inner.poll());
            self.max_pending
                .set(self.max_pending.get().max(self.held.len()));
        }
        fn poll(&mut self) -> Vec<super::super::MeshResult> {
            if self.release.get() {
                std::mem::take(&mut self.held)
            } else {
                vec![]
            }
        }
    }

    #[test]
    fn async_mesher_matches_inline_and_drops_stale_results() {
        let origin: ChunkPos = ivec3(0, 0, 0).into();
        let mut inline = World::new(GeneratorV0::new());
        settle(&mut inline, origin);

        let release = std::rc::Rc::new(std::cell::Cell::new(false));
        let max_pending = std::rc::Rc::new(std::cell::Cell::new(0));
        let mesher = DelayedMesher {
            release: release.clone(),
            max_pending: max_pending.clone(),
            ..Default::default()
        };
        let mut w = World::with_mesher(GeneratorV0::new(), Box::new(mesher));
        for _ in 0..200 {
            w.update(origin);
        }
        assert!(!w.is_settled(), "jobs still in flight");
        assert!(w.meshes.is_empty());
        assert!(max_pending.get() <= MAX_IN_FLIGHT);

        // walk far away while jobs are held: the old results must not resurrect chunks
        for x in 1..=30 {
            w.update(ivec3(x, 0, 0).into());
        }
        release.set(true);
        settle(&mut w, ivec3(30, 0, 0).into());
        assert!(w.is_settled());
        assert!(
            w.meshes
                .keys()
                .all(|c| ChunkPos::from(ivec3(30, 0, 0)).distance_squared(*c)
                    <= UNLOAD_RADIUS_SQUARED)
        );

        // back at the origin every in-range mesh equals the inline one
        // (retained far chunks depend on the walk path, so compare the load sphere only)
        settle(&mut w, origin);
        let mut checked = 0;
        for pos in w.meshes.keys() {
            if origin.distance_squared(*pos) <= LOAD_RADIUS_SQUARED {
                let a = w.mesh(*pos).unwrap();
                let b = inline.mesh(*pos).expect("inline has it");
                assert_eq!(a.vertex_data, b.vertex_data);
                checked += 1;
            }
        }
        assert!(checked > 100);
    }
}
