//! Meshing as a pure `bytes -> words` job, so it can run in a Web Worker.
//!
//! Input bytes: `BLOCKS_PER_CHUNK` blocks of the chunk, then for each side (in `Side`
//! order) the `CHUNK_SIZE`² cells of the neighbour touching that face (zeros = air or
//! unavailable). Output words: 6 per-side quad counts, then the quads of each side.
use std::fmt::Debug;

use glam::U16Vec3;

use super::{Chunk, ChunkDrawData, ChunkPos, Neighbours, BLOCKS_PER_CHUNK, CHUNK_SIZE};

const PLANE: usize = CHUNK_SIZE * CHUNK_SIZE;
pub const MESH_INPUT_BYTES: usize = BLOCKS_PER_CHUNK + 6 * PLANE;

/// Cell `(u, v)` of the neighbour plane touching side `side` (the plane the mesher reads).
fn plane_cell(side: usize, u: usize, v: usize) -> U16Vec3 {
    let (u, v, last) = (u as u16, v as u16, CHUNK_SIZE as u16 - 1);
    match side {
        0 => U16Vec3::new(u, 0, v),    // Top: neighbour's bottom plane
        1 => U16Vec3::new(u, last, v), // Bottom: neighbour's top plane
        2 => U16Vec3::new(u, v, 0),    // Front (z+)
        3 => U16Vec3::new(u, v, last), // Back (z-)
        4 => U16Vec3::new(0, u, v),    // Right (x+)
        _ => U16Vec3::new(last, u, v), // Left (x-)
    }
}

pub fn encode_mesh_input(chunk: &Chunk, neighbours: &Neighbours) -> Vec<u8> {
    let mut out = Vec::with_capacity(MESH_INPUT_BYTES);
    out.extend(chunk.iter_blocks().map(|b| b as u8));
    for (side, n) in neighbours.iter().enumerate() {
        for v in 0..CHUNK_SIZE {
            for u in 0..CHUNK_SIZE {
                out.push(n.map_or(0, |c| c.get_block(plane_cell(side, u, v)) as u8));
            }
        }
    }
    out
}

/// Mesh an `encode_mesh_input` buffer. Same quads as `Chunk::to_vertex_data` on the originals.
pub fn mesh_input(input: &[u8]) -> Vec<u32> {
    assert_eq!(input.len(), MESH_INPUT_BYTES, "bad mesh input size");
    let chunk = Chunk::from_block_bytes(&input[..BLOCKS_PER_CHUNK]);
    let planes: Vec<Option<Chunk>> = (0..6)
        .map(|side| {
            let p = &input[BLOCKS_PER_CHUNK + side * PLANE..][..PLANE];
            p.iter().any(|b| *b != 0).then(|| {
                let mut c = Chunk::empty();
                for v in 0..CHUNK_SIZE {
                    for u in 0..CHUNK_SIZE {
                        c.set(plane_cell(side, u, v), p[v * CHUNK_SIZE + u].into());
                    }
                }
                c
            })
        })
        .collect();
    let neighbours: Neighbours = std::array::from_fn(|i| planes[i].as_ref());
    let sides = chunk.to_vertex_data(&neighbours);
    let mut out: Vec<u32> = sides.iter().map(|s| s.len() as u32).collect();
    out.extend(sides.iter().flatten().map(|w| *w as u32));
    out
}

pub fn decode_mesh_output(words: &[u32]) -> ChunkDrawData {
    let mut rest = &words[6..];
    let vertex_data = std::array::from_fn(|i| {
        let (side, tail) = rest.split_at(words[i] as usize);
        rest = tail;
        side.iter().map(|w| *w as i32).collect()
    });
    ChunkDrawData { vertex_data }
}

#[derive(Debug)]
pub struct MeshJob {
    pub id: u64,
    pub pos: ChunkPos,
    pub input: Vec<u8>,
}

#[derive(Debug)]
pub struct MeshResult {
    pub id: u64,
    pub output: Vec<u32>,
}

/// Runs mesh jobs somewhere (inline, a worker...). Results may arrive in any frame.
pub trait MeshExecutor: Debug {
    fn submit(&mut self, job: MeshJob);
    /// Finished jobs since the last call.
    fn poll(&mut self) -> Vec<MeshResult>;
}

/// Meshes on `submit`: results are ready on the next `poll`. Native tests and the e2e harness.
#[derive(Debug, Default)]
pub struct InlineMesher {
    done: Vec<MeshResult>,
}

impl MeshExecutor for InlineMesher {
    fn submit(&mut self, job: MeshJob) {
        self.done.push(MeshResult {
            id: job.id,
            output: mesh_input(&job.input),
        });
    }

    fn poll(&mut self) -> Vec<MeshResult> {
        std::mem::take(&mut self.done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{BlockType, GeneratorV0, WorldGenerator};
    use fastrand::Rng;

    fn assert_roundtrip(c: &Chunk, n: &Neighbours) {
        let input = encode_mesh_input(c, n);
        assert_eq!(input.len(), MESH_INPUT_BYTES);
        let got = decode_mesh_output(&mesh_input(&input));
        assert_eq!(got.vertex_data, c.to_vertex_data(n));
    }

    #[test]
    fn job_matches_direct_meshing_random() {
        let mut rng = Rng::with_seed(7);
        for _ in 0..4 {
            let c = Chunk::random(&mut rng);
            let ns: Vec<Chunk> = (0..6).map(|_| Chunk::random(&mut rng)).collect();
            let n: Neighbours = std::array::from_fn(|i| Some(&ns[i]));
            assert_roundtrip(&c, &n);
            assert_roundtrip(&c, &[None; 6]);
            let mut some = [None; 6];
            some[2] = Some(&ns[2]);
            some[5] = Some(&ns[5]);
            assert_roundtrip(&c, &some);
        }
    }

    #[test]
    fn job_matches_direct_meshing_terrain_and_uniform() {
        let g = GeneratorV0::new();
        let below = g.generate(glam::ivec3(0, -1, 0).into());
        let here = g.generate(glam::ivec3(0, 0, 0).into());
        let right = g.generate(glam::ivec3(1, 0, 0).into());
        let n: Neighbours = [None, Some(&below), None, None, Some(&right), None];
        assert_roundtrip(&here, &n);
        let stone = Chunk::plain(BlockType::Stone);
        assert_roundtrip(&stone, &[Some(&stone), None, None, Some(&here), None, None]);
        assert_roundtrip(&Chunk::empty(), &[None; 6]);
    }
}
