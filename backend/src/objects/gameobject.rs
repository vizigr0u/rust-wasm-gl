use std::rc::Rc;

use glam::IVec3;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glow::HasContext;
use glow::WebTextureKey;

use crate::core::Time;
use crate::graphics::Camera;
use crate::graphics::MeshRenderer;
use crate::graphics::TextureDef;
use crate::graphics::TextureType;
use crate::graphics::UniformTypes;
use crate::world::ChunkPos;
use crate::world::WorldPos;
use crate::world::CHUNK_SIZE;

pub trait Transform {
    fn get_position(&self) -> WorldPos;
    fn set_position(&mut self, position: WorldPos);
    fn get_rotation(&self) -> Quat;
    fn set_rotation(&mut self, rotation: Quat);
    fn get_scale(&self) -> Vec3;
    fn set_scale(&mut self, scale: Vec3);
    fn get_velocity(&self) -> Vec3;
    fn set_velocity(&mut self, velocity: Vec3);

    fn get_block_position(&self) -> IVec3 {
        self.get_position().offset().floor().as_ivec3()
            + self.get_position().chunk_pos().as_vec() * CHUNK_SIZE as i32
    }

    fn get_chunk_position(&self) -> IVec3 {
        self.get_position().chunk_pos().as_vec()
    }
}

#[derive(Debug)]
pub struct GameObject {
    position: WorldPos,
    rotation: Quat,
    scale: Vec3,
    velocity: Vec3,
    renderer: Rc<MeshRenderer>,

    texture: Rc<TextureDef>,
}

impl GameObject {
    pub fn new(texture: &Rc<TextureDef>, renderer: &Rc<MeshRenderer>) -> Self {
        GameObject {
            position: WorldPos::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            texture: texture.clone(),
            renderer: renderer.clone(),
            velocity: Vec3::ZERO,
        }
    }

    pub fn update(&mut self, time: &Time) {
        if time.delta_time() > 0.0 && self.velocity != Vec3::ZERO {
            self.position += self.velocity * time.delta_time() as f32 * 0.001;
        }
    }

    pub fn _get_renderer(&self) -> &Rc<MeshRenderer> {
        &self.renderer
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera, player_chunk: ChunkPos) {
        unsafe {
            let program = self.renderer.get_program();
            program.gl_use(gl);

            program.set_matrix(gl, UniformTypes::ViewMatrix, &camera.look_at);
            program.set_matrix(gl, UniformTypes::ProjMatrix, &camera.projection);
            let transform = Mat4::from_scale_rotation_translation(
                self.scale,
                self.rotation,
                self.position.to_screen_pos(player_chunk),
            );
            program.set_matrix(gl, UniformTypes::ModelMatrix, &transform);

            let (tex_type, key) = *self.texture;
            if key != WebTextureKey::default() {
                let texture = Some(key);
                match tex_type {
                    TextureType::Texture2D => gl.bind_texture(glow::TEXTURE_2D, texture),
                    TextureType::Texture2DArray(_depth) => {
                        gl.bind_texture(glow::TEXTURE_2D_ARRAY, texture)
                    }
                };
            }

            self.renderer.render(gl);
        }
    }
}

impl Transform for GameObject {
    fn get_position(&self) -> WorldPos {
        self.position
    }

    fn set_position(&mut self, position: WorldPos) {
        self.position = position;
    }

    fn get_rotation(&self) -> Quat {
        self.rotation
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
    }

    fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
    }

    fn get_scale(&self) -> Vec3 {
        self.scale
    }

    fn get_velocity(&self) -> Vec3 {
        self.velocity
    }

    fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }
}
