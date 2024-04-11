use std::fmt::Debug;
use std::rc::Rc;

use super::RenderFlag;
use super::Renderer;

#[derive(Debug)]
pub enum MeshRenderVertex<R>
where
    R: Renderer,
{
    None,
    Array(Option<R::VertexArrayKey>, usize),
    Elements(Option<R::VertexArrayKey>, Option<R::BufferKey>, usize),
}

const DEFAULT_FLAGS: [RenderFlag; 3] = [
    RenderFlag::CullFace,
    RenderFlag::Blend,
    RenderFlag::DepthTest,
];

// pub trait MeshRenderer<R, S> {
//     fn new(program: Rc<S>) -> Self;
//     fn render(&self, mesh: &R::Mesh, renderer: &R, flags: &[RenderFlag]) -> R::Mesh;
// }

// pub struct MeshRendererData<R, S>
// where
//     R: Renderer,
// {
//     program: Rc<S>,
//     primitive_type: u32,
//     display_data: MeshRenderVertex<R>,
//     render_flags: &'static [RenderFlag], // vertex_count: i32,
// }
