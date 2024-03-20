use std::rc::Rc;

use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glow::HasContext;
use glow::WebTextureKey;

use crate::camera::Camera;
use crate::material::TextureDef;
use crate::material::TextureType;
use crate::meshrenderer::MeshRenderer;
use crate::shaders::UniformTypes;

#[derive(Debug)]
pub struct GameObject {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    renderer: Rc<MeshRenderer>,

    transform: Mat4,
    transform_dirty: bool,
    texture: Rc<TextureDef>,
}

impl GameObject {
    pub fn new(texture: &Rc<TextureDef>, renderer: &Rc<MeshRenderer>) -> Self {
        GameObject {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            texture: texture.clone(),
            transform: Mat4::IDENTITY,
            renderer: renderer.clone(),
            transform_dirty: true,
        }
    }

    pub fn update(&mut self) {
        if self.transform_dirty {
            self.transform =
                Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);
        }
    }

    pub fn _get_position(&self) -> Vec3 {
        self.position
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.transform_dirty = true;
    }

    pub fn _get_rotation(&self) -> Quat {
        self.rotation
    }

    pub fn _set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
        self.transform_dirty = true;
    }

    pub fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
        self.transform_dirty = true;
    }

    pub fn _get_renderer(&self) -> &Rc<MeshRenderer> {
        &self.renderer
    }

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
