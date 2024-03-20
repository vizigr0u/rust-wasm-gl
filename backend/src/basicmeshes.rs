#![allow(dead_code)]
#![allow(unused_variables)]

use glam::Vec3;

use crate::{
    chunk::BlockSideTexture,
    mesh::{uv_definitions, Mesh, Side, SideVertices, VertexAttrType},
};

pub fn make_quad() -> Mesh {
    let verts = [
        (A, uv_definitions::TOP_LEFT),
        (B, uv_definitions::TOP_RIGHT),
        (E, uv_definitions::BOT_LEFT),
        (F, uv_definitions::BOT_RIGHT),
    ];
    let mut data = Vec::<f32>::with_capacity(verts.len() * 5);
    for (pos, uv) in &verts {
        data.push(pos.x);
        data.push(pos.y);
        data.push(pos.z);
        data.push(uv.0 as _);
        data.push(uv.1 as _);
    }
    Mesh {
        data,
        indices: None,
        layout: vec![(VertexAttrType::Position, 3), (VertexAttrType::UVs, 2)],
        primitive_type: glow::TRIANGLE_STRIP,
    }
}

// . .
// . .
pub fn make_quad_triangles() -> Mesh {
    let verts = [
        (A, uv_definitions::TOP_LEFT),
        (B, uv_definitions::TOP_RIGHT),
        (E, uv_definitions::BOT_LEFT),
        (B, uv_definitions::TOP_RIGHT),
        (F, uv_definitions::BOT_RIGHT),
        (E, uv_definitions::BOT_LEFT),
    ];
    let mut data = Vec::<f32>::with_capacity(verts.len() * 5);
    for (pos, uv) in &verts {
        data.push(pos.x);
        data.push(pos.y);
        data.push(pos.z);
        data.push(uv.0 as _);
        data.push(uv.1 as _);
    }
    Mesh {
        data,
        indices: None,
        layout: vec![(VertexAttrType::Position, 3), (VertexAttrType::UVs, 2)],
        primitive_type: glow::TRIANGLES,
    }
}

pub fn make_quad_elements() -> Mesh {
    let verts = [
        (A, uv_definitions::TOP_LEFT),
        (B, uv_definitions::TOP_RIGHT),
        (E, uv_definitions::BOT_LEFT),
        (F, uv_definitions::BOT_RIGHT),
    ];
    let mut data = Vec::<f32>::with_capacity(verts.len() * 5);
    for (pos, uv) in &verts {
        data.push(pos.x);
        data.push(pos.y);
        data.push(pos.z);
        data.push(uv.0 as _);
        data.push(uv.1 as _);
    }
    Mesh {
        data,
        indices: Some(vec![0, 1, 2, 1, 3, 2]),
        layout: vec![(VertexAttrType::Position, 3), (VertexAttrType::UVs, 2)],
        primitive_type: glow::TRIANGLES,
    }
}

pub fn make_cube() -> Mesh {
    Mesh::from_offset_sides(
        [
            (Side::Top, BlockSideTexture::GrassTop, Vec3::ZERO),
            (Side::Bottom, BlockSideTexture::Dirt, Vec3::ZERO),
            (Side::Front, BlockSideTexture::GrassSide, Vec3::ZERO),
            (Side::Back, BlockSideTexture::GrassSide, Vec3::ZERO),
            (Side::Left, BlockSideTexture::GrassSide, Vec3::ZERO),
            (Side::Right, BlockSideTexture::GrassSide, Vec3::ZERO),
        ],
        1.0,
        &SIDE_VERTICES,
    )
}

const SIDE_VERTICES: [SideVertices; 6] = [
    SideVertices {
        a: C,
        b: A,
        c: B,
        d: D,
    },
    SideVertices {
        a: E,
        b: G,
        c: H,
        d: F,
    },
    SideVertices {
        a: A,
        b: E,
        c: F,
        d: B,
    },
    SideVertices {
        a: D,
        b: H,
        c: G,
        d: C,
    },
    SideVertices {
        a: B,
        b: F,
        c: H,
        d: D,
    },
    SideVertices {
        a: C,
        b: G,
        c: E,
        d: A,
    },
];

//   C        D
// A        B

//   G        H
// E        F

const A: Vec3 = Vec3 {
    x: -1.0,
    y: 1.0,
    z: 1.0,
};
const B: Vec3 = Vec3 {
    x: 1.0,
    y: 1.0,
    z: 1.0,
};
const C: Vec3 = Vec3 {
    x: -1.0,
    y: 1.0,
    z: -1.0,
};
const D: Vec3 = Vec3 {
    x: 1.0,
    y: 1.0,
    z: -1.0,
};
const E: Vec3 = Vec3 {
    x: -1.0,
    y: -1.0,
    z: 1.0,
};
const F: Vec3 = Vec3 {
    x: 1.0,
    y: -1.0,
    z: 1.0,
};
const G: Vec3 = Vec3 {
    x: -1.0,
    y: -1.0,
    z: -1.0,
};
const H: Vec3 = Vec3 {
    x: 1.0,
    y: -1.0,
    z: -1.0,
};
