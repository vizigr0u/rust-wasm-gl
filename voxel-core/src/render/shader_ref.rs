//! CPU reference of `backend/shaders/chunk.wgsl`: the contract for the WGSL port.
//! Output is pre view/projection (world-relative position).
use glam::{IVec3, Vec2, Vec3};

/// `mesh_pos`: base quad corner (slot 0), `data`: greedy quad word (instance, see `pack_quad`),
/// `side`: `Side as i32` from the draw's slot, `chunk_pos` / `player_chunk`: the shader computes the
/// chunk origin relative to the player chunk in integers (exact in f32). Returns (position, uv, texture layer).
/// The quad spans `w` along its `a` axis and `h` along `b`; UVs repeat over `0..w`, `0..h`.
pub fn chunk_vert_ref(
    mesh_pos: Vec3,
    data: i32,
    side: i32,
    chunk_pos: IVec3,
    player_chunk: IVec3,
) -> (Vec3, Vec2, i32) {
    let x = data & 15;
    let y = (data >> 4) & 15;
    let z = (data >> 8) & 15;
    let depth = (data >> 12) & 63;
    let a = (((data >> 18) & 15) + 1) as f32;
    let b = (((data >> 22) & 15) + 1) as f32;

    let position = Vec3::new(x as f32, y as f32, z as f32);
    let m = mesh_pos;
    let mut uvs = Vec2::new(m.z, m.x);
    let mut p = m;
    let mut size = Vec3::ONE;
    let mut uv_scale = Vec2::new(a, b);
    match side {
        0 => {
            p = Vec3::new(m.x, 1.0, 1.0 - m.z);
            uvs = Vec2::new(uvs.y, 1.0 - uvs.x);
            size = Vec3::new(a, 1.0, b);
        }
        1 => {
            uvs = Vec2::new(uvs.y, 1.0 - uvs.x);
            size = Vec3::new(a, 1.0, b);
        }
        2 => {
            p = Vec3::new(m.x, m.z, 1.0);
            uvs = Vec2::new(uvs.y, 1.0 - uvs.x);
            size = Vec3::new(a, b, 1.0);
        }
        3 => {
            p = Vec3::new(m.x, 1.0 - m.z, 0.0);
            uvs = Vec2::new(1.0 - uvs.y, uvs.x);
            size = Vec3::new(a, b, 1.0);
        }
        4 => {
            p = Vec3::new(1.0, m.x, m.z);
            uvs = Vec2::new(1.0 - uvs.x, 1.0 - uvs.y);
            size = Vec3::new(1.0, a, b);
            uv_scale = Vec2::new(b, a);
        }
        5 => {
            p = Vec3::new(0.0, m.x, 1.0 - m.z);
            uvs = Vec2::new(1.0 - uvs.x, 1.0 - uvs.y);
            size = Vec3::new(1.0, a, b);
            uv_scale = Vec2::new(b, a);
        }
        _ => {}
    }
    let chunk_offset = ((chunk_pos - player_chunk) * 16).as_vec3();
    (p * size + position + chunk_offset, uvs * uv_scale, depth)
}

/// Quad word, corner of the base quad (strip order) and draw slot for a pulled vertex.
/// Mirrors `vs_main` in `chunk.wgsl`: `vertex_index` is `base_vertex + index value`
/// (`4 * quad + corner`), `instance_index` is the draw slot `chunk_slot * 6 + side`.
pub fn chunk_vert_pulled(
    vertex_index: u32,
    instance_index: u32,
    quads: &[i32],
    chunks: &[IVec3],
    player_chunk: IVec3,
) -> (Vec3, Vec2, i32) {
    let base = crate::world::make_base_quad_data();
    let quad = (vertex_index >> 2) as usize;
    let corner = (vertex_index & 3) as usize;
    let chunk_slot = (instance_index / 6) as usize;
    let side = (instance_index % 6) as i32;
    chunk_vert_ref(
        base[corner],
        quads[quad],
        side,
        chunks[chunk_slot],
        player_chunk,
    )
}

/// Two triangles per quad, `4k+{0,1,2, 2,1,3}`: the strip `0,1,2,3` with the same winding.
pub const QUAD_INDEX_PATTERN: [u16; 6] = [0, 1, 2, 2, 1, 3];

/// WGSL text of the base quad corners, pasted into `chunk.wgsl`.
pub fn base_quad_wgsl() -> String {
    let c = crate::world::make_base_quad_data();
    let v: Vec<String> = c
        .iter()
        .map(|v| format!("vec3<f32>({:?}, {:?}, {:?})", v.x, v.y, v.z))
        .collect();
    format!("const BASE_QUAD = array<vec3<f32>, 4>({});", v.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::SIDES;
    use crate::world::{
        make_base_quad_data, pack_quad, side_quad_strip, BlockSideTexture, Side, SIDE_NORMS,
    };
    use glam::{U16Vec3, Vec3};

    fn word(off: u16, w: u32, h: u32) -> i32 {
        pack_quad(U16Vec3::splat(off), BlockSideTexture::GrassTop as i32, w, h)
    }

    #[test]
    fn golden_table() {
        let base = make_base_quad_data();
        let mut rows = Vec::new();
        for side in SIDES {
            for (off, w, h) in [(0u16, 1, 1), (15, 1, 1), (0, 16, 3), (2, 5, 16)] {
                for (i, v) in base.iter().enumerate() {
                    let (p, uv, l) =
                        chunk_vert_ref(*v, word(off, w, h), side as i32, IVec3::ZERO, IVec3::ZERO);
                    rows.push(format!(
                        "{side:?} off={off} {w}x{h} c{i}: pos=({:.5},{:.5},{:.5}) uv=({:.5},{:.5}) layer={l}",
                        p.x, p.y, p.z, uv.x, uv.y
                    ));
                }
            }
        }
        insta::assert_yaml_snapshot!(rows);
    }

    #[test]
    fn unit_quads_equal_side_vertices() {
        // Same 4 corners as SIDE_VERTICES (strip order is the shader's, winding test covers it).
        let key = |v: Vec3| (v.x as i32, v.y as i32, v.z as i32);
        let base = make_base_quad_data();
        for side in SIDES {
            for off in [0u16, 15] {
                let mut got: Vec<_> = base
                    .iter()
                    .map(|b| {
                        key(chunk_vert_ref(
                            *b,
                            word(off, 1, 1),
                            side as i32,
                            IVec3::ZERO,
                            IVec3::ZERO,
                        )
                        .0)
                    })
                    .collect();
                let mut want: Vec<_> = side_quad_strip(side)
                    .iter()
                    .map(|v| key(*v + Vec3::splat(off as f32)))
                    .collect();
                got.sort();
                want.sort();
                assert_eq!(got, want, "{side:?} off={off}");
            }
        }
    }

    #[test]
    fn wide_quad_is_union_of_unit_quads_and_uvs_repeat() {
        use crate::world::expand_to_legacy_words;
        let base = make_base_quad_data();
        for side in SIDES {
            let (w, h) = (5u32, 3u32);
            let big = word(1, w, h);
            let verts: Vec<_> = base
                .iter()
                .map(|b| chunk_vert_ref(*b, big, side as i32, IVec3::ZERO, IVec3::ZERO))
                .collect();
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for (p, _, _) in &verts {
                lo = lo.min(*p);
                hi = hi.max(*p);
            }
            let mut cell_lo = Vec3::splat(f32::MAX);
            let mut cell_hi = Vec3::splat(f32::MIN);
            let mut uv_max = Vec2::ZERO;
            for l in expand_to_legacy_words(side, big) {
                // legacy word: unit quad of one cell (side in word is the same as `side`)
                let unit = pack_quad(
                    U16Vec3::new((l & 15) as _, ((l >> 4) & 15) as _, ((l >> 8) & 15) as _),
                    BlockSideTexture::GrassTop as i32,
                    1,
                    1,
                );
                for b in &base {
                    let (p, uv, _) =
                        chunk_vert_ref(*b, unit, side as i32, IVec3::ZERO, IVec3::ZERO);
                    cell_lo = cell_lo.min(p);
                    cell_hi = cell_hi.max(p);
                    assert!(uv.x == 0.0 || uv.x == 1.0);
                }
            }
            for (_, uv, _) in &verts {
                uv_max = uv_max.max(*uv);
            }
            assert_eq!((lo, hi), (cell_lo, cell_hi), "{side:?} extent");
            let want = if matches!(side, Side::Right | Side::Left) {
                Vec2::new(h as f32, w as f32)
            } else {
                Vec2::new(w as f32, h as f32)
            };
            assert_eq!(uv_max, want, "{side:?} uv repeat");
        }
    }

    #[test]
    fn strip_winding_points_outward() {
        let base = make_base_quad_data();
        for side in SIDES {
            for (w, h) in [(1, 1), (7, 4)] {
                let v: Vec<Vec3> = base
                    .iter()
                    .map(|b| {
                        chunk_vert_ref(*b, word(0, w, h), side as i32, IVec3::ZERO, IVec3::ZERO).0
                    })
                    .collect();
                let n = SIDE_NORMS[side as usize];
                // strip tris: (v0,v1,v2) and (v2,v1,v3)
                for (a, b, c) in [(v[0], v[1], v[2]), (v[2], v[1], v[3])] {
                    assert!((b - a).cross(c - a).dot(n) > 0.0, "{side:?} winding");
                }
            }
        }
    }

    #[test]
    fn integer_offset_equals_cpu_chunk_offset() {
        use crate::render::chunk_offset;
        let base = make_base_quad_data();
        let word = word(3, 2, 2);
        for (c, p) in [
            ((2, 0, -1), (1, 0, 1)),
            ((-40, 3, 100), (-41, -2, 99)),
            ((0, 0, 0), (0, 0, 0)),
        ] {
            let (c, p) = (IVec3::from(c), IVec3::from(p));
            let off = chunk_offset(c.into(), p.into());
            for side in SIDES {
                for b in &base {
                    let new = chunk_vert_ref(*b, word, side as i32, c, p);
                    let old = chunk_vert_ref(*b, word, side as i32, IVec3::ZERO, IVec3::ZERO);
                    assert_eq!(new.0, old.0 + off);
                    assert_eq!((new.1, new.2), (old.1, old.2));
                }
            }
        }
    }

    #[test]
    fn side_order_matches_shader_constants() {
        // SideUp..SideLeft = 0..5 in chunk.wgsl
        let order = [
            Side::Top,
            Side::Bottom,
            Side::Front,
            Side::Back,
            Side::Right,
            Side::Left,
        ];
        for (i, s) in order.iter().enumerate() {
            assert_eq!(*s as usize, i);
            assert_eq!(SIDES[i], *s);
        }
    }

    #[test]
    fn wgsl_base_quad_matches_rust() {
        let src = include_str!("../../../backend/shaders/chunk.wgsl");
        assert!(
            src.contains(&base_quad_wgsl()),
            "expected: {}",
            base_quad_wgsl()
        );
    }

    #[test]
    fn pulled_equals_instanced_path() {
        let base = make_base_quad_data();
        let quads: Vec<i32> = (0..40u16)
            .map(|i| {
                pack_quad(
                    U16Vec3::new(i % 16, (i * 7) % 16, (i * 3) % 16),
                    (i % 60) as i32,
                    1 + (i % 16) as u32,
                    1 + ((i * 5) % 16) as u32,
                )
            })
            .collect();
        let chunks = [
            IVec3::new(2, 0, -1),
            IVec3::new(-40, 3, 100),
            IVec3::new(0, -1, 5),
        ];
        let player = IVec3::new(1, 0, 1);
        for slot in 0..(chunks.len() as u32 * 6) {
            let (chunk, side) = ((slot / 6) as usize, (slot % 6) as i32);
            for q in 0..quads.len() as u32 {
                // vertex_index as the GPU forms it: base_vertex + index value
                for (k, idx) in QUAD_INDEX_PATTERN.iter().enumerate() {
                    let vi = 4 * q + *idx as u32;
                    let got = chunk_vert_pulled(vi, slot, &quads, &chunks, player);
                    let want = chunk_vert_ref(
                        base[*idx as usize],
                        quads[q as usize],
                        side,
                        chunks[chunk],
                        player,
                    );
                    assert_eq!(got, want, "slot {slot} quad {q} k {k}");
                }
            }
        }
    }

    #[test]
    fn index_pattern_matches_strip_triangles() {
        // strip 0,1,2,3 -> tris (0,1,2) and (2,1,3)
        assert_eq!(QUAD_INDEX_PATTERN, [0, 1, 2, 2, 1, 3]);
    }
}

#[cfg(test)]
mod winding_tests {
    use super::*;
    use crate::render::SIDES;
    use crate::world::{make_base_quad_data, pack_quad, BlockSideTexture, SIDE_NORMS};
    use glam::U16Vec3;

    /// CCW front face: triangle `0,1,2` of the strip must have its normal along the side normal.
    #[test]
    fn triangles_wind_ccw_toward_side_normal() {
        let base = make_base_quad_data();
        let w = pack_quad(U16Vec3::ZERO, BlockSideTexture::GrassTop as i32, 3, 2);
        for side in SIDES {
            let p: Vec<Vec3> = base
                .iter()
                .map(|b| chunk_vert_ref(*b, w, side as i32, IVec3::ZERO, IVec3::ZERO).0)
                .collect();
            for (a, b, c) in [(0, 1, 2), (2, 1, 3)] {
                let n = (p[b] - p[a]).cross(p[c] - p[a]);
                assert!(
                    n.dot(SIDE_NORMS[side as usize]) > 0.0,
                    "{side:?} tri {a}{b}{c} normal {n:?}"
                );
            }
        }
    }
}
