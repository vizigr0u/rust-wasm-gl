use std::rc::Rc;

use glam::Vec3;
use log::info;

use super::{GameObject, LazyRenderGameObject, MakeRenderer};

use crate::{
    core::{HandleInputs, Time},
    graphics::{Camera, MeshRenderer, ShaderDef, ShaderProgram, UniformTypes, VertexAttrType},
    objects::gameobject::Transform,
    shader_def, utils,
    world::WorldPos,
};

const MAX_SPEED: f32 = 10.0;
const PLAYER_SCALE: f32 = 0.5;

#[derive(Debug, Default)]
pub struct CreatePlayerRenderer {
    origin: WorldPos,
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

#[derive(Debug)]
pub struct Player(LazyRenderGameObject<CreatePlayerRenderer>);

impl Default for Player {
    fn default() -> Self {
        Player(LazyRenderGameObject {
            gameobject: None,
            renderer_creator: Default::default(),
        })
    }
}

impl Player {
    pub fn new(origin: Vec3) -> Player {
        Player(LazyRenderGameObject {
            gameobject: None,
            renderer_creator: CreatePlayerRenderer {
                origin: WorldPos::ZERO + origin,
            },
        })
    }

    // pub fn get_gameobject(&self) -> &LazyRenderGameObject<CreatePlayerRenderer> {
    //     &self.0
    // }

    pub fn update(&mut self, time: &Time) {
        self.0.update(time, self.get_position().chunk_pos());
    }

    pub fn get_screen_position(&self) -> Vec3 {
        match &self.0.gameobject {
            Some(go) => go.get_position().offset(),
            None => Vec3::ZERO,
        }
    }

    pub fn render(&mut self, gl: &glow::Context, camera: &Camera) {
        self.0.render_lazy(gl, camera);
    }

    pub fn get_position(&self) -> WorldPos {
        match &self.0.gameobject {
            Some(go) => go.get_position(),
            None => WorldPos::ZERO,
        }
    }
}

impl HandleInputs for Player {
    fn handle_inputs(&mut self, inputs: &crate::core::InputState) {
        if let Some(go) = self.0.gameobject.as_mut() {
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
