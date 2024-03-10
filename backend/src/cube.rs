use glam::vec3;
use glam::Quat;
use glam::Vec3;

use crate::mesh::Mesh;
use crate::mesh::MeshDisplayType;
use crate::mesh::VertexAttrType;

pub struct Cube {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color: Vec3,
}

impl Cube {
    pub unsafe fn new() -> Self {
        Cube {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE * 0.5,
            color: vec3(1.0, 1.0, 1.0),
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
