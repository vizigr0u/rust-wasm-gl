use glam::{Vec2, Vec3};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VertexAttrType {
    Position,
    Color,
    Normal,
    UVs,
    Depth,
}

pub struct Mesh {
    pub data: Vec<f32>,
    pub indices: Option<Vec<u32>>,
    pub layout: Vec<(VertexAttrType, usize)>,
    pub primitive_type: u32,
}

impl Mesh {
    pub fn get_data(&self) -> &Vec<f32> {
        &self.data
    }
    pub fn make_quad() -> Mesh {
        let verts = [(A, TOP_LEFT), (B, TOP_RIGHT), (E, BOT_LEFT), (F, BOT_RIGHT)];
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
            (A, TOP_LEFT),
            (B, TOP_RIGHT),
            (E, BOT_LEFT),
            (B, TOP_RIGHT),
            (F, BOT_RIGHT),
            (E, BOT_LEFT),
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
        let verts = [(A, TOP_LEFT), (B, TOP_RIGHT), (E, BOT_LEFT), (F, BOT_RIGHT)];
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
        let sides = [
            (Side::Top, Texture::GrassTop),
            (Side::Bottom, Texture::Dirt),
            (Side::Front, Texture::GrassSide),
            (Side::Back, Texture::GrassSide),
            (Side::Left, Texture::GrassSide),
            (Side::Right, Texture::GrassSide),
        ];
        let mut data = Vec::<f32>::with_capacity(sides.len() * 4 * 9);
        for (side, texture) in &sides {
            for (vert, uv) in SIDE_VERTICES[*side as usize].get_quad_triangles() {
                let norm = &SIDE_NORMS[*side as usize];
                data.push(vert.x);
                data.push(vert.y);
                data.push(vert.z);
                data.push(uv.0 as _);
                data.push(uv.1 as _);
                data.push(norm.x);
                data.push(norm.y);
                data.push(norm.z);
                data.push(*texture as u8 as _);
            }
        }
        Mesh {
            data,
            layout: vec![
                (VertexAttrType::Position, 3),
                (VertexAttrType::UVs, 2),
                (VertexAttrType::Normal, 3),
                (VertexAttrType::Depth, 1),
            ],
            indices: None,
            primitive_type: glow::TRIANGLES,
        }
    }
}

#[derive(Clone, Copy)]
enum Side {
    Top = 0,
    Bottom,
    Front,
    Back,
    Right,
    Left,
}

const SIDE_NORMS: [Vec3; 6] = [
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
    Vec3::X,
    Vec3::NEG_X,
];

const SIDE_VERTICES: [SideVertices; 6] = [
    SideVertices {
        a: C,
        b: A,
        c: B,
        d: D,
    },
    SideVertices {
        a: A,
        b: E,
        c: F,
        d: B,
    },
    SideVertices {
        a: E,
        b: G,
        c: H,
        d: F,
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
    SideVertices {
        a: D,
        b: H,
        c: G,
        d: C,
    },
];

type Norm = Vec3;

impl Into<Norm> for Side {
    fn into(self) -> Norm {
        SIDE_NORMS[self as usize]
    }
}

impl Into<&'static SideVertices> for Side {
    fn into(self) -> &'static SideVertices {
        &SIDE_VERTICES[self as usize]
    }
}

struct SideVertices {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub d: Vec3,
}

impl SideVertices {
    pub fn get_quad_triangles(&self) -> [(Vec3, (i8, i8)); 6] {
        [
            (self.a, TOP_LEFT),
            (self.b, BOT_LEFT),
            (self.c, BOT_RIGHT),
            (self.a, TOP_LEFT),
            (self.c, BOT_RIGHT),
            (self.d, TOP_RIGHT),
        ]
    }
}

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

const BOT_LEFT: (i8, i8) = (0, 1);
const BOT_RIGHT: (i8, i8) = (1, 1);
const TOP_LEFT: (i8, i8) = (0, 0);
const TOP_RIGHT: (i8, i8) = (1, 0);

#[derive(Clone, Copy)]
enum Texture {
    Unknown = 0,
    GrassSide,
    Cobblestone,
    RedStone,
    TreeBark,
    Sand,
    Dirt,
    Pickaxe,
    TreeCenter,
    GrassTop,
    Coal,
    Lava,
    Diamond,
    Iron,
    Gold,
    Dirt2,
}
