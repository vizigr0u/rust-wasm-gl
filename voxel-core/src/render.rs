//! Renderer-agnostic helpers shared by every backend.
use glam::Vec3;

use crate::world::{ChunkDrawData, ChunkPos, Side, CHUNK_SIZE};

pub const SIDES: [Side; 6] = [
    Side::Top,
    Side::Bottom,
    Side::Front,
    Side::Back,
    Side::Right,
    Side::Left,
];

/// (first quad, quad count) per `Side`.
pub type SideRanges = [(u32, u32); 6];

/// Concatenate all 6 sides into one buffer plus the per-side ranges.
pub fn pack_sides(data: &ChunkDrawData) -> (Vec<i32>, SideRanges) {
    let mut all = Vec::with_capacity(data.quad_count());
    let mut ranges = [(0, 0); 6];
    for side in SIDES {
        let quads = &data.vertex_data[side as usize];
        ranges[side as usize] = (all.len() as u32, quads.len() as u32);
        all.extend_from_slice(quads);
    }
    (all, ranges)
}

// INTENTIONAL: cull faces relative to the *player* chunk, not the camera.
// The orbit camera is a debug view: holes seen from far away show what the
// player's own view culls. Do not "fix" (bugs.md #19) until a player-attached
// camera exists; then this debug camera can keep showing the culling.
pub fn is_visible(player_chunk: ChunkPos, chunk: ChunkPos, side: Side) -> bool {
    let (p, c) = (player_chunk.as_vec(), chunk.as_vec());
    match side {
        Side::Front => p.z >= c.z,
        Side::Back => p.z - 1 <= c.z,
        Side::Left => p.x - 1 <= c.x,
        Side::Right => p.x >= c.x,
        Side::Top => p.y >= c.y,
        Side::Bottom => p.y - 1 <= c.y,
    }
}

/// Floating origin: chunk offset from the player's chunk, exact in f32.
pub fn chunk_offset(chunk: ChunkPos, player_chunk: ChunkPos) -> Vec3 {
    ((chunk.as_vec() - player_chunk.as_vec()) * CHUNK_SIZE as i32).as_vec3()
}

/// One instanced draw: `count` quads starting at quad `first` of the chunk buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkDraw {
    pub chunk: ChunkPos,
    pub side: Side,
    pub offset: Vec3,
    pub first: u32,
    pub count: u32,
}

fn key(p: ChunkPos) -> (i32, i32, i32) {
    let v = p.as_vec();
    (v.x, v.y, v.z)
}

/// Visible draws, sorted by `ChunkPos` (then `Side`) so output is deterministic.
pub fn build_draw_list(
    chunks: impl Iterator<Item = (ChunkPos, SideRanges)>,
    player_chunk: ChunkPos,
) -> Vec<ChunkDraw> {
    let mut chunks: Vec<_> = chunks.collect();
    chunks.sort_by_key(|(p, _)| key(*p));
    let mut out = Vec::new();
    for (chunk, ranges) in chunks {
        let offset = chunk_offset(chunk, player_chunk);
        for side in SIDES {
            let (first, count) = ranges[side as usize];
            if count == 0 || !is_visible(player_chunk, chunk, side) {
                continue;
            }
            out.push(ChunkDraw {
                chunk,
                side,
                offset,
                first,
                count,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::ivec3;

    #[test]
    fn pack_ranges_tile_buffer() {
        let d = ChunkDrawData {
            vertex_data: [vec![1, 2], vec![], vec![3], vec![4, 5, 6], vec![], vec![7]],
        };
        let (all, r) = pack_sides(&d);
        assert_eq!(all, vec![1, 2, 3, 4, 5, 6, 7]);
        let mut next = 0;
        for (first, count) in r {
            assert_eq!(first, next);
            next += count;
        }
        assert_eq!(next as usize, all.len());
    }

    #[test]
    fn offset_is_chunk_delta() {
        let o = chunk_offset(ivec3(2, 0, -1).into(), ivec3(1, 0, 1).into());
        assert_eq!(o, Vec3::new(16.0, 0.0, -32.0));
    }

    #[test]
    fn draw_list_sorted_and_culled() {
        let r = [(0, 1); 6];
        let chunks = [ivec3(1, 0, 0), ivec3(-1, 0, 0)].map(|v| (v.into(), r));
        let list = build_draw_list(chunks.into_iter(), ivec3(0, 0, 0).into());
        assert!(list.windows(2).all(|w| key(w[0].chunk) <= key(w[1].chunk)));
        // player at 0: chunk x=1 hides Right (0>=1 false), chunk x=-1 hides Left (-1<=-1 true)
        assert!(!list.iter().any(|d| d.chunk.as_vec().x == 1 && matches!(d.side, Side::Right)));
        assert!(list.iter().any(|d| d.chunk.as_vec().x == -1 && matches!(d.side, Side::Right)));
    }
}
