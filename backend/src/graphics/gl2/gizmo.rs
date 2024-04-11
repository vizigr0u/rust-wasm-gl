use std::rc::Rc;

use glam::Vec3;
use log::info;

use crate::graphics::{RenderFlag, Renderer, ShaderFile};
use crate::{
    core::{
        position::{self, WorldPos},
        Transform,
    },
    glsl_def,
    graphics::{Mesh, PrimitiveType, ShaderDef, UniformTypes, VertexAttrType},
};

use super::{Gl2Renderer, GlShaderProgram, Glsl};

#[derive(Debug)]
pub struct Gizmo {
    origin: WorldPos,
    size: f32,
}

impl Gizmo {
    fn make_renderer(
        &self,
        gl: &Gl2Renderer,
    ) -> Result<Rc<MeshRenderer<Gl2Renderer, GlShaderProgram>>, String> {
        let program = compile_shader(gl)?;
        let mut renderer = MeshRenderer::with_render_flags(&GIZMO_GL_PARAMS, &program);
        let mesh = Mesh {
            data: GIZMO_VERTICES.to_vec(),
            layout: vec![(VertexAttrType::Position, 3), (VertexAttrType::Color, 3)],
            indices: None,
            primitive_type: PrimitiveType::Lines,
        };
        renderer.set_mesh(gl, Rc::new(mesh))?;
        Ok(Rc::new(renderer))
    }
}

impl Default for Gizmo {
    fn default() -> Self {
        Self {
            origin: WorldPos::ZERO,
            size: 1.0,
        }
    }
}

const GIZMO_GL_PARAMS: [RenderFlag; 1] = [RenderFlag::DepthTest];

const GIZMO_VERTICES: [f32; 36] = [
    0.0, 0.0, 0.0, 1.0, 0.0, 0.0, //
    1.0, 0.0, 0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 0.0, 1.0, //
    0.0, 0.0, 1.0, 0.0, 0.0, 1.0, //
];

fn compile_shader(r: &Gl2Renderer) -> Result<GlShaderProgram, String> {
    let def = glsl_def!(
        "gizmo.vert",
        "gizmo.frag",
        vec!(
            (VertexAttrType::Position, "position"),
            (VertexAttrType::Color, "color")
        ),
        vec!(
            (UniformTypes::ModelMatrix, "model"),
            (UniformTypes::ViewMatrix, "view"),
            (UniformTypes::ProjMatrix, "projection"),
        )
    );
    let program = def.compile(r)?;
    Ok(program)
}
