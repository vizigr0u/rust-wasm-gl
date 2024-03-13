use glam::Vec3;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VertexAttrType {
    Position,
    Color,
    Normal,
    UVs,
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
        let verts = [
            (C, TOP_LEFT, Vec3::Y), // Top
            (A, BOT_LEFT, Vec3::Y),
            (B, BOT_RIGHT, Vec3::Y),
            (C, TOP_LEFT, Vec3::Y),
            (B, BOT_RIGHT, Vec3::Y),
            (D, TOP_RIGHT, Vec3::Y),
            (A, TOP_LEFT, Vec3::Z), // Front
            (E, BOT_LEFT, Vec3::Z),
            (F, BOT_RIGHT, Vec3::Z),
            (A, TOP_LEFT, Vec3::Z),
            (F, BOT_RIGHT, Vec3::Z),
            (B, TOP_RIGHT, Vec3::Z),
            (E, TOP_LEFT, Vec3::NEG_Y), // Bottom
            (G, BOT_LEFT, Vec3::NEG_Y),
            (H, BOT_RIGHT, Vec3::NEG_Y),
            (E, TOP_LEFT, Vec3::NEG_Y),
            (H, BOT_RIGHT, Vec3::NEG_Y),
            (F, TOP_RIGHT, Vec3::NEG_Y),
            (B, TOP_LEFT, Vec3::X), // Right
            (F, BOT_LEFT, Vec3::X),
            (H, BOT_RIGHT, Vec3::X),
            (B, TOP_LEFT, Vec3::X),
            (H, BOT_RIGHT, Vec3::X),
            (D, TOP_RIGHT, Vec3::X),
            (C, TOP_LEFT, Vec3::NEG_X), // Left
            (G, BOT_LEFT, Vec3::NEG_X),
            (E, BOT_RIGHT, Vec3::NEG_X),
            (C, TOP_LEFT, Vec3::NEG_X),
            (E, BOT_RIGHT, Vec3::NEG_X),
            (A, TOP_RIGHT, Vec3::NEG_X),
            (D, TOP_LEFT, Vec3::NEG_Z), // Back
            (H, BOT_LEFT, Vec3::NEG_Z),
            (G, BOT_RIGHT, Vec3::NEG_Z),
            (D, TOP_LEFT, Vec3::NEG_Z),
            (G, BOT_RIGHT, Vec3::NEG_Z),
            (C, TOP_RIGHT, Vec3::NEG_Z),
        ];
        let mut data = Vec::<f32>::with_capacity(verts.len() * 8);
        for (pos, uv, normal) in &verts {
            data.push(pos.x);
            data.push(pos.y);
            data.push(pos.z);
            data.push(uv.0 as _);
            data.push(uv.1 as _);
            data.push(normal.x);
            data.push(normal.y);
            data.push(normal.z);
        }
        Mesh {
            data,
            layout: vec![
                (VertexAttrType::Position, 3),
                (VertexAttrType::UVs, 2),
                (VertexAttrType::Normal, 3),
            ],
            indices: None,
            primitive_type: glow::TRIANGLES,
        }
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

const BOT_LEFT: (i8, i8) = (0, 0);
const BOT_RIGHT: (i8, i8) = (1, 0);
const TOP_LEFT: (i8, i8) = (0, 1);
const TOP_RIGHT: (i8, i8) = (1, 1);
