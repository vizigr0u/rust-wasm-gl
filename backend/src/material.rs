use std::rc::Rc;

use glam::Vec3;
use glow::WebTextureKey;

use crate::shaders::CompiledShader;

#[derive(Clone, Copy)]
pub enum TextureType {
    Texture2D,
    Texture2DArray,
}

#[derive(Clone)]
pub struct Material {
    pub color: Vec3,
    pub texture: Option<(TextureType, WebTextureKey)>,
    shader: Rc<CompiledShader>,
}

impl Material {
    pub fn from_shader(shader: &Rc<CompiledShader>) -> Self {
        Material {
            shader: shader.clone(),
            color: Vec3::ONE,
            texture: None,
        }
    }
    pub fn get_shader(&self) -> &CompiledShader {
        &self.shader
    }
}
