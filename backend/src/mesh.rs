use glam::Vec3;
use log::info;

use crate::chunk::BlockSideTexture;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VertexAttrType {
    Position,
    Color,
    Normal,
    UVs,
    Depth,
}

pub type QuadSideData = (Side, BlockSideTexture, Vec3);

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

    pub fn from_offset_sides<I>(
        sides: I,
        block_size: f32,
        side_vertices: &[SideVertices; 6],
    ) -> Mesh
    where
        I: IntoIterator<Item = QuadSideData>,
        I::IntoIter: ExactSizeIterator,
    {
        let iterator = sides.into_iter();
        let size = iterator.len();
        let mut data = Vec::<f32>::with_capacity(size * 4 * 9);
        for (side, texture, offset) in iterator {
            let origin = Vec3::new(
                offset.x * block_size,
                offset.y * block_size,
                offset.z * block_size,
            );
            for (vert, uv) in side_vertices[side as usize].get_quad_triangles() {
                let norm = &SIDE_NORMS[side as usize];
                let pos = vert * block_size + origin;
                data.push(pos.x);
                data.push(pos.y);
                data.push(pos.z);
                data.push(uv.0 as _);
                data.push(uv.1 as _);
                data.push(norm.x);
                data.push(norm.y);
                data.push(norm.z);
                data.push(texture as usize as _);
            }
        }
        info!(
            "created Mesh with {} vertices from {} faces.",
            data.len() / 9,
            size
        );
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
pub enum Side {
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

type Norm = Vec3;

impl Into<Norm> for Side {
    fn into(self) -> Norm {
        SIDE_NORMS[self as usize]
    }
}

pub struct SideVertices {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub d: Vec3,
}

impl SideVertices {
    pub fn get_quad_triangles(&self) -> [(Vec3, (i8, i8)); 6] {
        [
            (self.a, uv_definitions::TOP_LEFT),
            (self.b, uv_definitions::BOT_LEFT),
            (self.c, uv_definitions::BOT_RIGHT),
            (self.a, uv_definitions::TOP_LEFT),
            (self.c, uv_definitions::BOT_RIGHT),
            (self.d, uv_definitions::TOP_RIGHT),
        ]
    }
}

pub mod uv_definitions {
    pub const BOT_LEFT: (i8, i8) = (0, 1);
    pub const BOT_RIGHT: (i8, i8) = (1, 1);
    pub const TOP_LEFT: (i8, i8) = (0, 0);
    pub const TOP_RIGHT: (i8, i8) = (1, 0);
}
