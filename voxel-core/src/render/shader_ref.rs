//! CPU reference of `backend/shaders/chunk.vert`: a line-by-line port used as the
//! contract for the WGSL port. Output is pre view/projection (world-relative position).
use glam::{Vec2, Vec3};

/// `mesh_pos`: base quad corner (slot 0), `data`: packed quad word (instance),
/// `chunk_offset`: chunk origin relative to the player chunk.
/// Returns (position, uv, texture layer).
pub fn chunk_vert_ref(mesh_pos: Vec3, data: i32, chunk_offset: Vec3) -> (Vec3, Vec2, i32) {
    let x = data & 15;
    let y = (data >> 4) & 15;
    let z = (data >> 8) & 15;
    let face = (data >> 12) & 7;
    let depth = (data >> 15) & 63;

    let position = Vec3::new(x as f32, y as f32, z as f32);
    let m = mesh_pos;
    let mut uvs = Vec2::new(m.z, m.x);
    let mut p = m;
    match face {
        0 => {
            p = Vec3::new(m.x, 1.0, 1.0 - m.z);
            uvs = Vec2::new(uvs.y, 1.0 - uvs.x);
        }
        1 => uvs = Vec2::new(uvs.y, 1.0 - uvs.x),
        2 => {
            p = Vec3::new(m.x, m.z, 1.0);
            uvs = Vec2::new(uvs.y, 1.0 - uvs.x);
        }
        3 => {
            p = Vec3::new(m.x, 1.0 - m.z, 0.0);
            uvs = Vec2::new(1.0 - uvs.y, uvs.x);
        }
        4 => {
            p = Vec3::new(1.0, m.x, m.z);
            uvs = Vec2::new(1.0 - uvs.x, 1.0 - uvs.y);
        }
        5 => {
            p = Vec3::new(0.0, m.x, 1.0 - m.z);
            uvs = Vec2::new(1.0 - uvs.x, 1.0 - uvs.y);
        }
        _ => {}
    }
    (p + position + chunk_offset, uvs, depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::SIDES;
    use crate::world::{
        make_base_quad_data, make_side_quad_data, side_quad_strip, BlockSideTexture, Side,
        SIDE_NORMS,
    };
    use glam::{U16Vec3, Vec3};

    fn word(side: Side, off: u16) -> i32 {
        make_side_quad_data(side, BlockSideTexture::GrassTop, U16Vec3::splat(off))
    }

    #[test]
    fn golden_table() {
        let base = make_base_quad_data();
        let mut rows = Vec::new();
        for side in SIDES {
            for off in [0u16, 15] {
                for (i, v) in base.iter().enumerate() {
                    let (p, uv, l) = chunk_vert_ref(*v, word(side, off), Vec3::ZERO);
                    rows.push(format!(
                        "{side:?} off={off} c{i}: pos=({:.5},{:.5},{:.5}) uv=({:.5},{:.5}) layer={l}",
                        p.x, p.y, p.z, uv.x, uv.y
                    ));
                }
            }
        }
        insta::assert_yaml_snapshot!(rows);
    }

    #[test]
    fn corners_equal_side_vertices() {
        // Same 4 corners as SIDE_VERTICES (strip order is the shader's, winding test covers it).
        let key = |v: Vec3| (v.x as i32, v.y as i32, v.z as i32);
        let base = make_base_quad_data();
        for side in SIDES {
            for off in [0u16, 15] {
                let mut got: Vec<_> = base
                    .iter()
                    .map(|b| key(chunk_vert_ref(*b, word(side, off), Vec3::ZERO).0))
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
    fn strip_winding_points_outward() {
        let base = make_base_quad_data();
        for side in SIDES {
            let v: Vec<Vec3> = base
                .iter()
                .map(|b| chunk_vert_ref(*b, word(side, 0), Vec3::ZERO).0)
                .collect();
            let n = SIDE_NORMS[side as usize];
            // strip tris: (v0,v1,v2) and (v2,v1,v3)
            for (a, b, c) in [(v[0], v[1], v[2]), (v[2], v[1], v[3])] {
                assert!((b - a).cross(c - a).dot(n) > 0.0, "{side:?} winding");
            }
        }
    }

    #[test]
    fn side_order_matches_shader_constants() {
        // SideUp..SideLeft = 0..5 in chunk.vert
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
            assert_eq!((word(*s, 0) >> 12) & 7, i as i32);
        }
    }
}
