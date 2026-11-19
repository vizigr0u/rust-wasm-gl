//! CPU reference of `backend/shaders/chunk.wgsl`: the contract for the WGSL port.
//! Output is pre view/projection (world-relative position).
use glam::{Vec2, Vec3};

/// `mesh_pos`: base quad corner (slot 0), `data`: greedy quad word (instance, see `pack_quad`),
/// `side`: `Side as i32` from the draw's range entry, `chunk_offset`: chunk origin relative to
/// the player chunk. Returns (position, uv, texture layer).
/// The quad spans `w` along its `a` axis and `h` along `b`; UVs repeat over `0..w`, `0..h`.
pub fn chunk_vert_ref(
    mesh_pos: Vec3,
    data: i32,
    side: i32,
    chunk_offset: Vec3,
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
    (p * size + position + chunk_offset, uvs * uv_scale, depth)
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
                    let (p, uv, l) = chunk_vert_ref(*v, word(off, w, h), side as i32, Vec3::ZERO);
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
                    .map(|b| key(chunk_vert_ref(*b, word(off, 1, 1), side as i32, Vec3::ZERO).0))
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
                .map(|b| chunk_vert_ref(*b, big, side as i32, Vec3::ZERO))
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
                    let (p, uv, _) = chunk_vert_ref(*b, unit, side as i32, Vec3::ZERO);
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
                    .map(|b| chunk_vert_ref(*b, word(0, w, h), side as i32, Vec3::ZERO).0)
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
}
