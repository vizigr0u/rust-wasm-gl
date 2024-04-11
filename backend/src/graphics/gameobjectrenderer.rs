use glam::IVec3;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glow::HasContext;
use glow::WebTextureKey;

use crate::core::position::ChunkPos;
use crate::core::position::WorldPos;
use crate::core::position::CHUNK_SIZE;
use crate::core::Time;
use crate::core::Transform;
use crate::graphics::TextureType;
use crate::graphics::UniformTypes;
use crate::objects::Camera;

use super::Renderer;

#[derive(Debug)]
pub trait GameObjectRenderer<R>
where
    R: Renderer,
{
}

impl<R> GameObjectRenderer<R>
where
    R: Renderer,
{
    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        unsafe {
            let program = self.renderer.get_program();
            program.gl_use(gl);

            program.set_matrix(gl, UniformTypes::ViewMatrix, &camera.look_at);
            program.set_matrix(gl, UniformTypes::ProjMatrix, &camera.projection);
            program.set_matrix(gl, UniformTypes::ModelMatrix, &self.transform);

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
