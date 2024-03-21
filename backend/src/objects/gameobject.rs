use std::rc::Rc;

use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glow::HasContext;
use glow::WebTextureKey;
use log::info;

use crate::core::Time;
use crate::graphics::Camera;
use crate::graphics::MeshRenderer;
use crate::graphics::TextureDef;
use crate::graphics::TextureType;
use crate::graphics::UniformTypes;

pub trait Transform {
    fn get_position(&self) -> Vec3;
    fn set_position(&mut self, position: Vec3);
    fn get_rotation(&self) -> Quat;
    fn set_rotation(&mut self, rotation: Quat);
    fn get_scale(&self) -> Vec3;
    fn set_scale(&mut self, scale: Vec3);
    fn get_velocity(&self) -> Vec3;
    fn set_velocity(&mut self, velocity: Vec3);
}

#[derive(Debug)]
pub struct GameObject {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    velocity: Vec3,
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
            velocity: Vec3::ZERO,
        }
    }

    pub fn update(&mut self, time: &Time) {
        if time.delta_time() > 0.0 {
            self.position += self.velocity * time.delta_time() as f32 * 0.001;
            info!("Position: {:?}", self.position);
            self.transform_dirty = true;
        }
        if self.transform_dirty {
            self.transform =
                Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);
        }
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

impl Transform for GameObject {
    fn get_position(&self) -> Vec3 {
        self.position
    }

    fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.transform_dirty = true;
    }

    fn get_rotation(&self) -> Quat {
        self.rotation
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
        self.transform_dirty = true;
    }

    fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
        self.transform_dirty = true;
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
