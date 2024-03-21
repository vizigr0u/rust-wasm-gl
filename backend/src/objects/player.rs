use std::rc::Rc;

use glam::Vec3;
use log::info;

use super::{GameObject, LazyRenderGameObject, MakeRenderer};

use crate::{
    core::HandleInputs,
    graphics::{MeshRenderer, ShaderDef, ShaderProgram, UniformTypes, VertexAttrType},
    objects::gameobject::Transform,
    shader_def, utils,
};

const MAX_SPEED: f32 = 10.0;
const PLAYER_SCALE: f32 = 0.5;

#[derive(Debug, Default)]
pub struct CreatePlayerRenderer {
    origin: Vec3,
}

impl MakeRenderer for CreatePlayerRenderer {
    fn make_renderer(&self, gl: &glow::Context) -> Result<Rc<MeshRenderer>, String> {
        let program = compile_shader(gl)?;
        let mut renderer = MeshRenderer::new(&program);
        let mesh = utils::make_cube();
        renderer.set_mesh(gl, Rc::new(mesh))?;
        Ok(Rc::new(renderer))
    }

    fn init_gameobject(&self, gameobject: &mut GameObject) {
        gameobject.set_position(self.origin);
        gameobject.set_scale(Vec3::ONE * PLAYER_SCALE);
        info!("Created Player at {:?}", self.origin);
    }
}

pub type Player = LazyRenderGameObject<CreatePlayerRenderer>;

impl Default for Player {
    fn default() -> Self {
        LazyRenderGameObject {
            gameobject: None,
            renderer_creator: Default::default(),
        }
    }
}

impl Player {
    pub fn new(origin: Vec3) -> LazyRenderGameObject<CreatePlayerRenderer> {
        LazyRenderGameObject {
            gameobject: None,
            renderer_creator: CreatePlayerRenderer { origin },
        }
    }
}

impl HandleInputs for Player {
    fn handle_inputs(&mut self, inputs: &crate::core::InputState) {
        if let Some(go) = self.gameobject.as_mut() {
            let mut velocity = Vec3::ZERO;
            if inputs.is_key_down("KeyW") {
                velocity.x = 1.0;
            }
            if inputs.is_key_down("KeyS") {
                velocity.x = -1.0;
            }
            if inputs.is_key_down("KeyD") {
                velocity.z = 1.0;
            }
            if inputs.is_key_down("KeyA") {
                velocity.z = -1.0;
            }
            go.set_velocity(velocity * MAX_SPEED);
        }
    }
}

fn compile_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
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
