//! Renderer-agnostic helpers shared by every backend.
pub mod shader_ref;

use glam::Vec3;

use crate::math::Frustum;

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
// +axis faces have their plane at v+1 (can reach p*16 when c = p-1) so `p >= c`;
// -axis faces have their plane at v (max p*16-1 when c = p-1, behind the eye) so `p <= c`.
pub fn is_visible(player_chunk: ChunkPos, chunk: ChunkPos, side: Side) -> bool {
    let (p, c) = (player_chunk.as_vec(), chunk.as_vec());
    match side {
        Side::Front => p.z >= c.z,
        Side::Back => p.z <= c.z,
        Side::Left => p.x <= c.x,
        Side::Right => p.x >= c.x,
        Side::Top => p.y >= c.y,
        Side::Bottom => p.y <= c.y,
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

/// Chunk AABB in player-chunk-relative space.
pub fn chunk_aabb(offset: Vec3) -> (Vec3, Vec3) {
    (offset, offset + Vec3::splat(CHUNK_SIZE as f32))
}

/// Split draws by frustum. `None` keeps everything. Also returns the offsets of the
/// culled chunks (once per chunk) for debug boxes.
pub fn cull_draws(draws: &[ChunkDraw], frustum: Option<&Frustum>) -> (Vec<ChunkDraw>, Vec<Vec3>) {
    let Some(f) = frustum else {
        return (draws.to_vec(), Vec::new());
    };
    let mut visible = Vec::with_capacity(draws.len());
    let mut culled: Vec<Vec3> = Vec::new();
    let mut last: Option<(ChunkPos, bool)> = None;
    for &d in draws {
        let vis = match last {
            Some((c, v)) if c == d.chunk => v,
            _ => {
                let (min, max) = chunk_aabb(d.offset);
                let v = f.intersects_aabb(min, max);
                if !v {
                    culled.push(d.offset);
                }
                last = Some((d.chunk, v));
                v
            }
        };
        if vis {
            visible.push(d);
        }
    }
    (visible, culled)
}

fn key(p: ChunkPos) -> (i32, i32, i32) {
    let v = p.as_vec();
    (v.x, v.y, v.z)
}

fn dist2(p: ChunkPos, player: ChunkPos) -> i64 {
    let d = p.as_vec() - player.as_vec();
    let d = d.as_i64vec3();
    d.x * d.x + d.y * d.y + d.z * d.z
}

/// Unculled draw list, rebuilt only when the player chunk changes or the
/// chunk set changes (`invalidate`).
#[derive(Default)]
pub struct DrawListCache {
    key: Option<ChunkPos>,
    draws: Vec<ChunkDraw>,
}

impl DrawListCache {
    pub fn invalidate(&mut self) {
        self.key = None;
    }

    pub fn get(
        &mut self,
        chunks: impl Iterator<Item = (ChunkPos, SideRanges)>,
        player_chunk: ChunkPos,
    ) -> &[ChunkDraw] {
        if self.key != Some(player_chunk) {
            self.draws = build_draw_list(chunks, player_chunk);
            self.key = Some(player_chunk);
        }
        &self.draws
    }
}

/// Visible draws, front to back (squared chunk distance to the player, then
/// `ChunkPos` key as tie-break, then `Side`) so output is deterministic.
pub fn build_draw_list(
    chunks: impl Iterator<Item = (ChunkPos, SideRanges)>,
    player_chunk: ChunkPos,
) -> Vec<ChunkDraw> {
    let mut chunks: Vec<_> = chunks.collect();
    chunks.sort_by_key(|(p, _)| (dist2(*p, player_chunk), key(*p)));
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
        let pc: ChunkPos = ivec3(0, 0, 0).into();
        assert!(list.windows(2).all(|w| {
            (dist2(w[0].chunk, pc), key(w[0].chunk)) <= (dist2(w[1].chunk, pc), key(w[1].chunk))
        }));
        // player at 0: chunk x=1 hides Right (0>=1 false), chunk x=-1 hides Left (0<=-1 false)
        assert!(!list
            .iter()
            .any(|d| d.chunk.as_vec().x == 1 && matches!(d.side, Side::Right)));
        assert!(list
            .iter()
            .any(|d| d.chunk.as_vec().x == -1 && matches!(d.side, Side::Right)));
    }

    #[test]
    fn cache_matches_fresh_build() {
        let r = [(0, 1); 6];
        let mut set: Vec<(ChunkPos, SideRanges)> = Vec::new();
        let mut cache = DrawListCache::default();
        let mut player: ChunkPos = ivec3(0, 0, 0).into();
        for step in 0..6 {
            match step {
                0 | 1 => set.push((ivec3(step + 1, 0, 0).into(), r)),
                2 => player = ivec3(2, 0, 0).into(),
                3 => {
                    set.remove(0);
                }
                4 => player = ivec3(-1, 0, 1).into(),
                _ => set.push((ivec3(5, 0, 5).into(), r)),
            }
            if matches!(step, 0 | 1 | 3 | 5) {
                cache.invalidate();
            }
            let fresh = build_draw_list(set.iter().copied(), player);
            assert_eq!(cache.get(set.iter().copied(), player), &fresh[..]);
        }
    }

    #[test]
    fn cull_splits_visible_and_culled() {
        let f = crate::playerview::PlayerView::new().frustum(Vec3::ZERO);
        let r = [(0, 1); 6];
        let player: ChunkPos = ivec3(0, 0, 0).into();
        let chunks = [ivec3(3, 0, 0), ivec3(-3, 0, 0)].map(|v| (v.into(), r));
        let all = build_draw_list(chunks.clone().into_iter(), ivec3(3, 0, 0).into());
        let n = all.len();
        let (vis, culled) = cull_draws(&all, None);
        assert_eq!((vis.len(), culled.len()), (n, 0));
        let draws = build_draw_list(chunks.into_iter(), player);
        let (vis, culled) = cull_draws(&draws, Some(&f));
        assert!(vis.iter().all(|d| d.chunk.as_vec().x == 3));
        assert!(!vis.is_empty());
        assert_eq!(culled, vec![Vec3::new(-48.0, 0.0, 0.0)]);
    }

    // ---- A4 snapshots ----
    use crate::testutil::hash_words;
    use crate::world::{GeneratorV0, World, MAX_CHUNK_Y, MIN_CHUNK_Y};

    #[test]
    fn is_visible_full_table() {
        let mut rows = Vec::new();
        for side in SIDES {
            let mut row = String::new();
            for d in -2..=2 {
                // offset of chunk relative to player, along the side's axis
                for axis in 0..3 {
                    let mut v = ivec3(0, 0, 0);
                    v[axis] = d;
                    let vis = is_visible(ivec3(0, 0, 0).into(), v.into(), side);
                    row.push(if vis { 'V' } else { '.' });
                }
                row.push(' ');
            }
            rows.push(format!("{side:?}: {row}"));
        }
        insta::assert_yaml_snapshot!(rows);
    }

    #[test]
    fn side_cull_is_mirror_symmetric() {
        let pairs = [
            (Side::Right, Side::Left, 0),
            (Side::Top, Side::Bottom, 1),
            (Side::Front, Side::Back, 2),
        ];
        for (pos, neg, axis) in pairs {
            for d in -3..=3 {
                let (mut a, mut b) = (ivec3(0, 0, 0), ivec3(0, 0, 0));
                a[axis] = d;
                b[axis] = -d;
                let o: ChunkPos = ivec3(0, 0, 0).into();
                assert_eq!(
                    is_visible(o, a.into(), pos),
                    is_visible(o, b.into(), neg),
                    "{pos:?} axis {axis} d {d}"
                );
            }
        }
    }

    #[test]
    fn no_back_faces_drawn_behind_player() {
        let r = [(0, 1); 6];
        let player: ChunkPos = ivec3(0, 0, 0).into();
        let mut chunks = Vec::new();
        for x in -2..=2 {
            for y in -2..=2 {
                for z in -2..=2 {
                    chunks.push((ivec3(x, y, z).into(), r));
                }
            }
        }
        for d in build_draw_list(chunks.into_iter(), player) {
            let c = d.chunk.as_vec();
            match d.side {
                Side::Bottom => assert!(c.y >= 0),
                Side::Back => assert!(c.z >= 0),
                Side::Left => assert!(c.x >= 0),
                _ => {}
            }
        }
    }

    #[test]
    fn pack_sides_snapshot() {
        let d = ChunkDrawData {
            vertex_data: [vec![1, 2], vec![], vec![3], vec![4, 5, 6], vec![], vec![7]],
        };
        insta::assert_yaml_snapshot!(pack_sides(&d));
    }

    #[test]
    fn draw_list_small_snapshot() {
        let r: SideRanges = [(0, 2), (2, 0), (2, 3), (5, 1), (6, 4), (10, 1)];
        let chunks = [ivec3(1, 0, 0), ivec3(-1, 0, 0), ivec3(0, 0, 0)].map(|v| (v.into(), r));
        let list = build_draw_list(chunks.into_iter(), ivec3(0, 0, 0).into());
        let out: Vec<String> = list
            .iter()
            .map(|d| {
                format!(
                    "{:?} {:?} off={:?} first={} count={}",
                    d.chunk.as_vec(),
                    d.side,
                    d.offset,
                    d.first,
                    d.count
                )
            })
            .collect();
        insta::assert_yaml_snapshot!(out);
    }

    #[test]
    fn draw_list_settled_world_summary() {
        let mut w = World::new(GeneratorV0::new());
        let player: ChunkPos = ivec3(0, 0, 0).into();
        let mut meshes: std::collections::HashMap<ChunkPos, (usize, SideRanges)> =
            Default::default();
        for _ in 0..2000 {
            let ch = w.update(player);
            for p in ch.removed {
                meshes.remove(&p);
            }
            for p in ch.added {
                let (all, r) = pack_sides(w.mesh(p).unwrap());
                meshes.insert(p, (all.len(), r));
            }
        }
        let list = build_draw_list(meshes.iter().map(|(p, (_, r))| (*p, *r)), player);
        let words: Vec<i32> = list
            .iter()
            .flat_map(|d| {
                let v = d.chunk.as_vec();
                [v.x, v.y, v.z, d.side as i32, d.first as i32, d.count as i32]
            })
            .collect();
        let total: u32 = list.iter().map(|d| d.count).sum();
        // chunks above/below world height are never meshed
        assert!(list
            .iter()
            .all(|d| (MIN_CHUNK_Y..MAX_CHUNK_Y).contains(&d.chunk.as_vec().y)));
        insta::assert_yaml_snapshot!((meshes.len(), list.len(), total, hash_words(&words)));
    }
}
