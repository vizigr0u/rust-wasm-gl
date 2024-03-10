use glam::Quat;
use glam::Vec3;

use crate::mesh::Mesh;
use crate::mesh::MeshDisplayType;
use crate::mesh::VertexAttrType;

pub struct Quad {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Quad {
    pub unsafe fn new() -> Self {
        Quad {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE * 0.5,
        }
    }

    pub fn make_mesh() -> Mesh {
        Mesh {
            data: vec![
                -1.0, -1.0, 0.0, 0.0, 1.0, //
                1.0, -1.0, 0.0, 1.0, 1.0, //
                -1.0, 1.0, 0.0, 0.0, 0.0, //
                1.0, 1.0, 0.0, 1.0, 0.0, //
            ],
            layout: vec![(VertexAttrType::Position, 3), (VertexAttrType::UVs, 2)],
            display_type: MeshDisplayType::TriangleStrip,
        }
    }
}
