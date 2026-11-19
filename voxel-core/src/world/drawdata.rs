use super::{Chunk, Neighbours};

/// Greedy-meshed quads of one chunk: one packed `i32` word per quad (`pack_quad`), grouped by `Side`.
#[derive(Debug, Clone)]
pub struct ChunkDrawData {
    pub vertex_data: [Vec<i32>; 6],
}

impl ChunkDrawData {
    pub fn quad_count(&self) -> usize {
        self.vertex_data.iter().map(Vec::len).sum()
    }

    pub fn load(chunk: &Chunk, neighbours: &Neighbours) -> Self {
        Self {
            vertex_data: chunk.to_vertex_data(neighbours),
        }
    }
}
