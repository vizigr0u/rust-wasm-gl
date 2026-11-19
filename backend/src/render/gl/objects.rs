use std::rc::Rc;

use glam::Mat4;

use super::{Mesh, MeshRenderer, ShaderDef, ShaderProgram, UniformTypes, VertexAttrType};
use crate::{shader_def, utils, utils::GlRenderFlags};

/// GL resources for the simple scene objects. Created eagerly once GL is ready;
/// objects are drawn by reference from plain game state.
#[derive(Debug)]
pub struct ObjectRenderers {
    cube: MeshRenderer,
    gizmo: MeshRenderer,
}

impl ObjectRenderers {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        let program = compile_cube_shader(gl)?;
        let mut cube = MeshRenderer::new(&program);
        cube.set_mesh(gl, Rc::new(utils::make_cube()))?;

        let program = compile_gizmo_shader(gl)?;
        let mut gizmo = MeshRenderer::with_render_flags(&GIZMO_GL_PARAMS, &program);
        let mesh = Mesh {
            data: GIZMO_VERTICES.to_vec(),
            layout: vec![(VertexAttrType::Position, 3), (VertexAttrType::Color, 3)],
            indices: None,
            primitive_type: glow::LINES,
        };
        gizmo.set_mesh(gl, Rc::new(mesh))?;

        Ok(ObjectRenderers { cube, gizmo })
    }

    /// Note: the cube shader samples whatever texture is currently bound
    /// (the block atlas, bound by the world draw). See codebase-review A10.
    pub fn draw_cube(&self, gl: &glow::Context, view: &Mat4, projection: &Mat4, model: &Mat4) {
        draw(gl, &self.cube, view, projection, model);
    }

    pub fn draw_gizmo(&self, gl: &glow::Context, view: &Mat4, projection: &Mat4, model: &Mat4) {
        draw(gl, &self.gizmo, view, projection, model);
    }
}

fn draw(gl: &glow::Context, renderer: &MeshRenderer, view: &Mat4, projection: &Mat4, model: &Mat4) {
    let program = renderer.get_program();
    program.gl_use(gl);
    program.set_matrix(gl, UniformTypes::ViewMatrix, view);
    program.set_matrix(gl, UniformTypes::ProjMatrix, projection);
    program.set_matrix(gl, UniformTypes::ModelMatrix, model);
    renderer.render(gl);
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

pub fn compile_cube_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
    unsafe {
        let program = shader_def!(
            "cube.vert",
            "cube.frag",
            vec!(
                (VertexAttrType::Position, "position"),
                (VertexAttrType::Normal, "normal"),
                (VertexAttrType::UVs, "uv"),
                (VertexAttrType::Depth, "depth"),
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

pub fn compile_gizmo_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
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
