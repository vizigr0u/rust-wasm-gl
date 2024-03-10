#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VertexAttrType {
    Position,
    Color,
    Normal,
    UVs,
}

#[derive(Clone, Copy)]
pub enum MeshDisplayType {
    Triangles = glow::TRIANGLES as _,
    TriangleStrip = glow::TRIANGLE_STRIP as _,
}

pub struct Mesh {
    pub data: Vec<f32>,
    pub layout: Vec<(VertexAttrType, usize)>,
    pub display_type: MeshDisplayType,
}

impl Mesh {
    pub fn get_data(&self) -> &Vec<f32> {
        &self.data
    }
}