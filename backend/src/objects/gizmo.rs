use std::rc::Rc;

use glam::Vec3;
use log::info;

use super::{GameObject, LazyRenderGameObject, MakeRenderer};

use crate::{
    core::Time,
    graphics::{Mesh, MeshRenderer, ShaderDef, ShaderProgram, UniformTypes, VertexAttrType},
    objects::gameobject::Transform,
    shader_def,
    utils::GlRenderFlags,
};

#[derive(Debug)]
pub struct CreateGizmoRenderer {
    origin: Vec3,
    size: f32,
}

impl MakeRenderer for CreateGizmoRenderer {
    fn make_renderer(&self, gl: &glow::Context) -> Result<Rc<MeshRenderer>, String> {
        let program = compile_shader(gl)?;
        let mut renderer = MeshRenderer::with_render_flags(&GIZMO_GL_PARAMS, &program);
        let mesh = Mesh {
            data: GIZMO_VERTICES.to_vec(),
            layout: vec![(VertexAttrType::Position, 3), (VertexAttrType::Color, 3)],
            indices: None,
            primitive_type: glow::LINES,
        };
        renderer.set_mesh(gl, Rc::new(mesh))?;
        Ok(Rc::new(renderer))
    }

    fn init_gameobject(&self, gameobject: &mut GameObject) {
        gameobject.set_position(self.origin);
        gameobject.set_scale(Vec3::ONE * self.size);
        info!("Created Gizmo at {:?}, size: {}", self.origin, self.size);
    }
}

pub type Gizmo = LazyRenderGameObject<CreateGizmoRenderer>;

impl Default for Gizmo {
    fn default() -> Self {
        Self::new(Vec3::ZERO, 1.0)
    }
}

impl Gizmo {
    pub fn new(origin: Vec3, size: f32) -> LazyRenderGameObject<CreateGizmoRenderer> {
        LazyRenderGameObject {
            gameobject: None,
            renderer_creator: CreateGizmoRenderer { origin, size },
        }
    }
}

const GIZMO_GL_PARAMS: [GlRenderFlags; 1] = [GlRenderFlags::DepthTest];

const GIZMO_VERTICES: [f32; 36] = [
    0.0, 0.0, 0.0, 1.0, 0.0, 0.0, //
    1.0, 0.0, 0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 0.0, 1.0, //
    0.0, 0.0, 1.0, 0.0, 0.0, 1.0, //
];

fn compile_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
    unsafe {
        let program = shader_def!(
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
        )
        .compile(gl)?;
        Ok(Rc::new(program))
    }
}
